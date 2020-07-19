use crate::sound::generator::Generators;
use crate::sound::hammond::mk_hammond;
use crate::sound::wave_form::WaveForm;
use crate::utils::thread_worker::ThreadWorker;
use crate::ErrorString;
use jack::*;

#[derive(Debug, PartialEq)]
struct HarmonicVolume {
    index: usize,
    volume: f32,
}

#[derive(Debug, PartialEq)]
enum MidiControllerEvent {
    Volume(f32),
    HarmonicVolume(HarmonicVolume),
}

impl MidiControllerEvent {
    fn convert_volume(byte: &u8) -> f32 {
        f32::min(1.0, *byte as f32 / 127.0)
    }

    fn from_raw_midi(event: RawMidi<'_>) -> Option<MidiControllerEvent> {
        match event.bytes {
            [176, 11, volume] => Some(MidiControllerEvent::Volume(
                MidiControllerEvent::convert_volume(volume),
            )),
            [176, slider @ 3..=10, volume] => {
                Some(MidiControllerEvent::HarmonicVolume(HarmonicVolume {
                    index: *slider as usize - 3,
                    volume: MidiControllerEvent::convert_volume(volume),
                }))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod from_raw_midi_to_midi_controller_event {
    use super::*;

    #[test]
    fn converts_the_controller_events_correctly() {
        let table = vec![
            // volume slider
            ([176, 11, 0], Some(MidiControllerEvent::Volume(0.0))),
            ([176, 11, 127], Some(MidiControllerEvent::Volume(1.0))),
            (
                [176, 11, 64],
                Some(MidiControllerEvent::Volume(64.0 / 127.0)),
            ),
            ([176, 11, 128], Some(MidiControllerEvent::Volume(1.0))),
            // first harmonic
            (
                [176, 3, 0],
                Some(MidiControllerEvent::HarmonicVolume(HarmonicVolume {
                    index: 0,
                    volume: 0.0,
                })),
            ),
            (
                [176, 3, 127],
                Some(MidiControllerEvent::HarmonicVolume(HarmonicVolume {
                    index: 0,
                    volume: 1.0,
                })),
            ),
            (
                [176, 3, 64],
                Some(MidiControllerEvent::HarmonicVolume(HarmonicVolume {
                    index: 0,
                    volume: 64.0 / 127.0,
                })),
            ),
            (
                [176, 3, 128],
                Some(MidiControllerEvent::HarmonicVolume(HarmonicVolume {
                    index: 0,
                    volume: 1.0,
                })),
            ),
            // second harmonic
            (
                [176, 4, 64],
                Some(MidiControllerEvent::HarmonicVolume(HarmonicVolume {
                    index: 1,
                    volume: 64.0 / 127.0,
                })),
            ),
            // eighth harmonic
            (
                [176, 10, 64],
                Some(MidiControllerEvent::HarmonicVolume(HarmonicVolume {
                    index: 7,
                    volume: 64.0 / 127.0,
                })),
            ),
            // unmapped events
            ([176, 1, 0], None),
            ([176, 2, 0], None),
            ([176, 12, 0], None),
            ([176, 13, 0], None),
        ];
        for (bytes, expected) in table {
            println!("bytes: {:?}, expected: {:?}", bytes, expected);
            let raw_midi = RawMidi {
                time: 0,
                bytes: &bytes,
            };
            assert_eq!(MidiControllerEvent::from_raw_midi(raw_midi), expected);
        }
    }
}

pub struct MidiController {
    port: Port<MidiIn>,
    event_handler: EventHandler,
}

impl MidiController {
    pub fn new(client: &Client) -> Result<MidiController, ErrorString> {
        Ok(MidiController {
            port: client.register_port("controller", MidiIn)?,
            event_handler: EventHandler::new(),
        })
    }

    pub fn handle_events(&self, generators: &mut Generators, scope: &ProcessScope) {
        self.event_handler
            .handle_events(generators, self.port.iter(scope));
    }
}

struct EventHandler {
    hammond_generator: ThreadWorker<HarmonicVolume, WaveForm>,
}

impl EventHandler {
    fn new() -> EventHandler {
        let mut harmonics_state = HarmonicsState::new();
        EventHandler {
            hammond_generator: ThreadWorker::new(move |harmonic_volume| {
                harmonics_state.set_harmonic_volume(harmonic_volume);
                harmonics_state.mk_wave_form()
            }),
        }
    }

    fn handle_events<'a, Iter>(&self, generators: &mut Generators, raw_events: Iter)
    where
        Iter: Iterator<Item = RawMidi<'a>>,
    {
        for raw_event in raw_events {
            if let Some(event) = MidiControllerEvent::from_raw_midi(raw_event) {
                self.handle_midi_controller_event(generators, event);
            }
        }
        self.poll_hammond_generator(generators);
    }

    fn handle_midi_controller_event(
        &self,
        generators: &mut Generators,
        event: MidiControllerEvent,
    ) {
        match event {
            MidiControllerEvent::Volume(volume) => generators.midi_controller_volume = volume,
            MidiControllerEvent::HarmonicVolume(values) => self.hammond_generator.enqueue(values),
        }
    }

    fn poll_hammond_generator(&self, generators: &mut Generators) {
        if let Some(new_wave_form) = self.hammond_generator.poll() {
            generators.wave_form = new_wave_form;
        }
    }
}

struct HarmonicsState {
    harmonics: [f32; 8],
}

impl HarmonicsState {
    fn new() -> HarmonicsState {
        HarmonicsState {
            harmonics: [0.0; 8],
        }
    }

    fn set_harmonic_volume(&mut self, HarmonicVolume { index, volume }: HarmonicVolume) {
        if index < self.harmonics.len() {
            self.harmonics[index] = volume;
        }
    }

    fn mk_wave_form(&self) -> WaveForm {
        mk_hammond(self.harmonics.to_vec(), 44100)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sound::generator::test::sine_generators;
    use crate::utils::thread_worker::test::wait_for;

    fn compare_wave_forms(a: &WaveForm, b: &WaveForm) -> Result<(), String> {
        if a == b {
            Ok(())
        } else {
            Err(format!(
                "{:?} /= {:?}",
                a.table.iter().take(5).collect::<Vec<_>>(),
                b.table.iter().take(5).collect::<Vec<_>>(),
            ))
        }
    }

    mod handle_events {
        use super::*;

        #[test]
        fn adjusts_midi_volume_in_generators() {
            let events = vec![RawMidi {
                time: 0,
                bytes: &[176, 11, 64],
            }];
            let mut generators = sine_generators();
            let event_handler = EventHandler::new();
            event_handler.handle_events(&mut generators, events.into_iter());
            assert_eq!(generators.midi_controller_volume, 64.0 / 127.0);
        }

        #[test]
        fn adjusts_wave_form_in_generators() -> Result<(), String> {
            let event_handler = EventHandler::new();
            let mut generators = sine_generators();
            let expected = mk_hammond(vec![42.0 / 127.0], generators.wave_form.table.len());
            let events = vec![RawMidi {
                time: 0,
                bytes: &[176, 3, 42],
            }];
            event_handler.handle_events(&mut generators, events.into_iter());
            wait_for(|| {
                event_handler.handle_events(&mut generators, vec![].into_iter());
                compare_wave_forms(&generators.wave_form, &expected)?;
                Ok(())
            })?;
            Ok(())
        }
    }

    mod handle_midi_controller_event {
        use super::*;

        #[test]
        fn adjusts_the_midi_controller_volume() {
            let mut generators = sine_generators();
            let event_handler = EventHandler::new();
            event_handler
                .handle_midi_controller_event(&mut generators, MidiControllerEvent::Volume(0.7));
            assert_eq!(generators.midi_controller_volume, 0.7);
        }
    }

    mod harmonics_state {
        use super::*;

        #[test]
        fn allows_to_control_the_first_harmonic() -> Result<(), String> {
            let mut harmonics_state = HarmonicsState::new();
            harmonics_state.set_harmonic_volume(HarmonicVolume {
                index: 0,
                volume: 0.7,
            });
            let result = harmonics_state.mk_wave_form();
            compare_wave_forms(&result, &mk_hammond(vec![0.7], result.table.len()))
        }

        #[test]
        fn allows_to_control_the_second_harmonic() -> Result<(), String> {
            let mut harmonics_state = HarmonicsState::new();
            harmonics_state.set_harmonic_volume(HarmonicVolume {
                index: 1,
                volume: 0.7,
            });
            let result = harmonics_state.mk_wave_form();
            compare_wave_forms(&result, &mk_hammond(vec![0.0, 0.7], result.table.len()))
        }

        #[test]
        fn allows_to_mix_multiple_harmonics() -> Result<(), String> {
            let mut harmonics_state = HarmonicsState::new();
            harmonics_state.set_harmonic_volume(HarmonicVolume {
                index: 0,
                volume: 1.0,
            });
            harmonics_state.set_harmonic_volume(HarmonicVolume {
                index: 1,
                volume: 0.4,
            });
            let result = harmonics_state.mk_wave_form();
            compare_wave_forms(&result, &mk_hammond(vec![1.0, 0.4], result.table.len()))
        }

        #[test]
        fn allows_up_to_eight_harmonics() -> Result<(), String> {
            let mut harmonics_state = HarmonicsState::new();
            for index in 0..8 {
                harmonics_state.set_harmonic_volume(HarmonicVolume { index, volume: 0.2 });
            }
            let result = harmonics_state.mk_wave_form();
            compare_wave_forms(
                &result,
                &mk_hammond(
                    vec![0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2],
                    result.table.len(),
                ),
            )
        }

        #[test]
        fn does_not_crash_on_out_of_bounds_inputs() {
            let mut harmonics_state = HarmonicsState::new();
            for index in 0..10 {
                harmonics_state.set_harmonic_volume(HarmonicVolume { index, volume: 0.1 });
            }
        }
    }
}
