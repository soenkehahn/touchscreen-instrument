use crate::sound::generator;
use crate::sound::generator::Generators;
use crate::sound::hammond::mk_hammond;
use crate::sound::wave_form::WaveForm;
use crate::utils::thread_worker::ThreadWorker;
use crate::ErrorString;
use jack::*;

#[derive(Debug, PartialEq)]
enum MidiControllerEvent {
    Volume(f32),
    Envelope(EnvelopeEvent),
    HarmonicVolume(HarmonicVolume),
}

#[derive(Debug, PartialEq)]
enum EnvelopeEvent {
    Attack(f32),
    Decay(f32),
    Sustain(f32),
    Release(f32),
}

#[derive(Debug, PartialEq)]
struct HarmonicVolume {
    index: usize,
    volume: f32,
}

impl MidiControllerEvent {
    fn midi_to_float(byte: u8) -> f32 {
        f32::min(1.0, byte as f32 / 127.0)
    }

    fn convert_to_range(min: f32, max: f32, byte: u8) -> f32 {
        MidiControllerEvent::midi_to_float(byte) * (max - min) + min
    }

    fn convert_to_volume_factor(byte: u8) -> f32 {
        let value = MidiControllerEvent::midi_to_float(byte);
        const B: f32 = 4.0;
        const ROLL_OFF_LIMIT: f32 = 0.1;
        // from https://www.dr-lex.be/info-stuff/volumecontrols.html
        let roll_off_factor = if value < ROLL_OFF_LIMIT {
            value / ROLL_OFF_LIMIT
        } else {
            1.0
        };
        let a = 1.0 / B.exp();
        f32::min(1.0, a * (value * B).exp() * roll_off_factor)
    }

    fn from_raw_midi(event: RawMidi<'_>) -> Option<MidiControllerEvent> {
        match event.bytes {
            [176, 11, volume] | [183, 1, volume] => Some(MidiControllerEvent::Volume(
                MidiControllerEvent::convert_to_volume_factor(*volume),
            )),
            [176, 14, value] => Some(MidiControllerEvent::Envelope(EnvelopeEvent::Attack(
                MidiControllerEvent::convert_to_range(
                    generator::MIN_ATTACK,
                    generator::MAX_ATTACK,
                    *value,
                ),
            ))),
            [176, 15, value] => Some(MidiControllerEvent::Envelope(EnvelopeEvent::Decay(
                MidiControllerEvent::convert_to_range(
                    generator::MIN_DECAY,
                    generator::MAX_DECAY,
                    *value,
                ),
            ))),
            [176, 16, value] => Some(MidiControllerEvent::Envelope(EnvelopeEvent::Sustain(
                MidiControllerEvent::convert_to_range(
                    generator::MIN_SUSTAIN,
                    generator::MAX_SUSTAIN,
                    *value,
                ),
            ))),
            [176, 17, value] => Some(MidiControllerEvent::Envelope(EnvelopeEvent::Release(
                MidiControllerEvent::convert_to_range(
                    generator::MIN_RELEASE,
                    generator::MAX_RELEASE,
                    *value,
                ),
            ))),
            [176, slider @ 3..=10, volume] => {
                Some(MidiControllerEvent::HarmonicVolume(HarmonicVolume {
                    index: *slider as usize - 3,
                    volume: MidiControllerEvent::convert_to_volume_factor(*volume),
                }))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod from_raw_midi_to_midi_controller_event {
    use super::*;

    mod range_to_volume_factor {
        use super::*;

        #[test]
        fn is_strictly_monotonic() {
            for (i, j) in (0..=126).zip(1..=127) {
                let previous = MidiControllerEvent::convert_to_volume_factor(i);
                let next = MidiControllerEvent::convert_to_volume_factor(j);
                assert!(
                    previous < next,
                    format!(
                        "not strictly monotonic: {} -> {}, {} -> {}",
                        i, previous, j, next
                    )
                )
            }
        }
    }

    #[test]
    fn converts_the_controller_events_correctly() {
        let table = vec![
            // volume slider
            ([176, 11, 0], Some(MidiControllerEvent::Volume(0.0))),
            ([176, 11, 127], Some(MidiControllerEvent::Volume(1.0))),
            (
                [176, 11, 64],
                Some(MidiControllerEvent::Volume(
                    MidiControllerEvent::convert_to_volume_factor(64),
                )),
            ),
            ([176, 11, 128], Some(MidiControllerEvent::Volume(1.0))),
            // volume pedal
            ([183, 1, 0], Some(MidiControllerEvent::Volume(0.0))),
            ([183, 1, 127], Some(MidiControllerEvent::Volume(1.0))),
            // envelope values
            (
                [176, 14, 0],
                Some(MidiControllerEvent::Envelope(EnvelopeEvent::Attack(
                    generator::MIN_ATTACK,
                ))),
            ),
            (
                [176, 14, 127],
                Some(MidiControllerEvent::Envelope(EnvelopeEvent::Attack(
                    generator::MAX_ATTACK,
                ))),
            ),
            (
                [176, 15, 0],
                Some(MidiControllerEvent::Envelope(EnvelopeEvent::Decay(
                    generator::MIN_DECAY,
                ))),
            ),
            (
                [176, 15, 127],
                Some(MidiControllerEvent::Envelope(EnvelopeEvent::Decay(
                    generator::MAX_DECAY,
                ))),
            ),
            (
                [176, 16, 0],
                Some(MidiControllerEvent::Envelope(EnvelopeEvent::Sustain(
                    generator::MIN_SUSTAIN,
                ))),
            ),
            (
                [176, 16, 127],
                Some(MidiControllerEvent::Envelope(EnvelopeEvent::Sustain(
                    generator::MAX_SUSTAIN,
                ))),
            ),
            (
                [176, 17, 0],
                Some(MidiControllerEvent::Envelope(EnvelopeEvent::Release(
                    generator::MIN_RELEASE,
                ))),
            ),
            (
                [176, 17, 127],
                Some(MidiControllerEvent::Envelope(EnvelopeEvent::Release(
                    generator::MAX_RELEASE,
                ))),
            ),
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
                    volume: MidiControllerEvent::convert_to_volume_factor(64),
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
                    volume: MidiControllerEvent::convert_to_volume_factor(64),
                })),
            ),
            // eighth harmonic
            (
                [176, 10, 64],
                Some(MidiControllerEvent::HarmonicVolume(HarmonicVolume {
                    index: 7,
                    volume: MidiControllerEvent::convert_to_volume_factor(64),
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
            MidiControllerEvent::Envelope(event) => match event {
                EnvelopeEvent::Attack(attack) => generators.envelope.attack = attack,
                EnvelopeEvent::Decay(decay) => generators.envelope.decay = decay,
                EnvelopeEvent::Sustain(sustain) => generators.envelope.sustain = sustain,
                EnvelopeEvent::Release(release) => generators.envelope.release = release,
            },
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
        mk_hammond(&self.harmonics, 44100)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::sound::generator::test::generators::sine_generators;
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
            assert_eq!(
                generators.midi_controller_volume,
                MidiControllerEvent::convert_to_volume_factor(64)
            );
        }

        #[test]
        fn adjusts_envelope_attack_values() {
            let events = vec![RawMidi {
                time: 0,
                bytes: &[176, 14, 127],
            }];
            let mut generators = sine_generators();
            EventHandler::new().handle_events(&mut generators, events.into_iter());
            assert_eq!(generators.envelope.attack, generator::MAX_ATTACK);
        }

        #[test]
        fn adjusts_envelope_decay_values() {
            let events = vec![RawMidi {
                time: 0,
                bytes: &[176, 15, 127],
            }];
            let mut generators = sine_generators();
            EventHandler::new().handle_events(&mut generators, events.into_iter());
            assert_eq!(generators.envelope.decay, generator::MAX_DECAY);
        }

        #[test]
        fn adjusts_envelope_sustain_values() {
            let events = vec![RawMidi {
                time: 0,
                bytes: &[176, 16, 0],
            }];
            let mut generators = sine_generators();
            EventHandler::new().handle_events(&mut generators, events.into_iter());
            assert_eq!(generators.envelope.sustain, generator::MIN_SUSTAIN);
        }

        #[test]
        fn adjusts_envelope_release_value() {
            let events = vec![RawMidi {
                time: 0,
                bytes: &[176, 17, 127],
            }];
            let mut generators = sine_generators();
            EventHandler::new().handle_events(&mut generators, events.into_iter());
            assert_eq!(generators.envelope.release, generator::MAX_RELEASE);
        }

        #[test]
        fn adjusts_wave_form_in_generators() -> Result<(), String> {
            let events = vec![RawMidi {
                time: 0,
                bytes: &[176, 3, 42],
            }];
            let mut generators = sine_generators();
            let event_handler = EventHandler::new();
            let expected = mk_hammond(
                &[MidiControllerEvent::convert_to_volume_factor(42)],
                generators.wave_form.table.len(),
            );
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
            compare_wave_forms(&result, &mk_hammond(&[0.7], result.table.len()))
        }

        #[test]
        fn allows_to_control_the_second_harmonic() -> Result<(), String> {
            let mut harmonics_state = HarmonicsState::new();
            harmonics_state.set_harmonic_volume(HarmonicVolume {
                index: 1,
                volume: 0.7,
            });
            let result = harmonics_state.mk_wave_form();
            compare_wave_forms(&result, &mk_hammond(&[0.0, 0.7], result.table.len()))
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
            compare_wave_forms(&result, &mk_hammond(&[1.0, 0.4], result.table.len()))
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
                    &[0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2],
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
