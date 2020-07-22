use crate::cli;
use crate::sound::wave_form::WaveForm;
use crate::sound::NoteEvent;
use crate::sound::{POLYPHONY, TAU};
use crate::utils::Slots;

struct Envelope {
    attack: f32,
    release: f32,
}

pub struct Generators {
    amplitude: f32,
    pub midi_controller_volume: f32,
    envelope: Envelope,
    pub wave_form: WaveForm,
    pub voices: Vec<VoiceState>,
}

impl Generators {
    pub fn new(cli_args: &cli::Args) -> Generators {
        let unit_slots: Slots<()> = [(); 10];
        let slots = unit_slots.len();
        Generators {
            amplitude: cli_args.volume / slots as f32,
            midi_controller_volume: 1.0,
            envelope: Envelope {
                attack: 0.005,
                release: 0.005,
            },
            wave_form: WaveForm::new(&cli_args.wave_form_config),
            voices: vec![VoiceState::default(); POLYPHONY],
        }
    }

    pub fn handle_note_events(&mut self, voices: [NoteEvent; POLYPHONY]) {
        for (i, event) in voices.iter().enumerate() {
            match event {
                NoteEvent::NoteOff { .. } => {
                    self.voices[i].note_off();
                }
                NoteEvent::NoteOn { frequency } => {
                    self.voices[i].note_on(*frequency);
                }
            }
        }
    }

    pub fn generate(&mut self, sample_rate: usize, buffer: &mut [f32]) {
        for voice in self.voices.iter_mut() {
            for sample in buffer.iter_mut() {
                voice.step(sample_rate, &self.envelope);
                match *voice {
                    VoiceState::Playing {
                        phase,
                        ref envelope_phase,
                        ..
                    } => {
                        let envelope_amplitude = match envelope_phase {
                            EnvelopePhase::Attacking { attack_amplitude } => *attack_amplitude,
                            EnvelopePhase::FullVolume => 1.0,
                            EnvelopePhase::Releasing { release_amplitude } => *release_amplitude,
                        };
                        *sample += self.wave_form.run(phase)
                            * self.amplitude
                            * self.midi_controller_volume
                            * envelope_amplitude;
                    }
                    VoiceState::Muted => {}
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum EnvelopePhase {
    Attacking { attack_amplitude: f32 },
    FullVolume,
    Releasing { release_amplitude: f32 },
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum VoiceState {
    Playing {
        frequency: f32,
        phase: f32,
        envelope_phase: EnvelopePhase,
    },
    Muted,
}

impl Default for VoiceState {
    fn default() -> VoiceState {
        VoiceState::Muted
    }
}

impl VoiceState {
    pub fn note_on(&mut self, frequency: f32) {
        *self = match *self {
            VoiceState::Playing {
                ref envelope_phase,
                phase,
                ..
            } => VoiceState::Playing {
                frequency,
                phase,
                envelope_phase: match envelope_phase {
                    EnvelopePhase::Attacking {
                        attack_amplitude, ..
                    } => EnvelopePhase::Attacking {
                        attack_amplitude: *attack_amplitude,
                    },
                    EnvelopePhase::FullVolume => EnvelopePhase::FullVolume,
                    EnvelopePhase::Releasing { .. } => EnvelopePhase::Attacking {
                        attack_amplitude: 0.0,
                    },
                },
            },
            VoiceState::Muted => VoiceState::Playing {
                frequency,
                phase: 0.0,
                envelope_phase: EnvelopePhase::Attacking {
                    attack_amplitude: 0.0,
                },
            },
        };
    }

    pub fn note_off(&mut self) {
        *self = match *self {
            VoiceState::Playing {
                frequency,
                phase,
                ref envelope_phase,
            } => VoiceState::Playing {
                frequency,
                phase,
                envelope_phase: EnvelopePhase::Releasing {
                    release_amplitude: match envelope_phase {
                        EnvelopePhase::Attacking { attack_amplitude } => *attack_amplitude,
                        EnvelopePhase::FullVolume => 1.0,
                        EnvelopePhase::Releasing { release_amplitude } => *release_amplitude,
                    },
                },
            },
            VoiceState::Muted => VoiceState::Muted,
        }
    }

    fn crank_phase(&mut self, sample_rate: usize) {
        match *self {
            VoiceState::Playing {
                frequency,
                ref mut phase,
                ..
            } => {
                *phase += frequency * TAU / sample_rate as f32;
                *phase %= TAU;
            }
            VoiceState::Muted => {}
        };
    }

    fn step_envelope(&mut self, sample_rate: usize, envelope: &Envelope) {
        let next = match *self {
            VoiceState::Playing {
                frequency,
                phase,
                ref mut envelope_phase,
            } => match envelope_phase {
                EnvelopePhase::Attacking {
                    ref mut attack_amplitude,
                } => {
                    *attack_amplitude += 1.0 / (sample_rate as f32 * envelope.attack);
                    if *attack_amplitude >= 1.0 {
                        Some(VoiceState::Playing {
                            frequency,
                            phase,
                            envelope_phase: EnvelopePhase::FullVolume,
                        })
                    } else {
                        None
                    }
                }
                EnvelopePhase::FullVolume => None,
                EnvelopePhase::Releasing {
                    ref mut release_amplitude,
                    ..
                } => {
                    *release_amplitude -= 1.0 / (sample_rate as f32 * envelope.release);
                    if *release_amplitude <= 0.0 {
                        Some(VoiceState::Muted)
                    } else {
                        None
                    }
                }
            },
            _ => None,
        };
        if let Some(n) = next {
            *self = n;
        }
    }

    fn step(&mut self, sample_rate: usize, envelope: &Envelope) {
        self.crank_phase(sample_rate);
        self.step_envelope(sample_rate, envelope);
    }
}

#[cfg(test)]
pub mod test {
    use super::*;

    const SAMPLE_RATE: usize = 44100;

    pub fn sine_generators() -> Generators {
        Generators {
            amplitude: 1.0,
            midi_controller_volume: 1.0,
            envelope: Envelope {
                attack: 0.0,
                release: 0.0,
            },
            wave_form: WaveForm::from_function(|x| x.sin(), SAMPLE_RATE),
            voices: vec![VoiceState::default(); POLYPHONY],
        }
    }

    fn monophonic_sine_generators() -> Generators {
        Generators {
            amplitude: 1.0,
            midi_controller_volume: 1.0,
            envelope: Envelope {
                attack: 0.0,
                release: 0.0,
            },
            wave_form: WaveForm::from_function(|x| x.sin(), SAMPLE_RATE),
            voices: vec![VoiceState::default()],
        }
    }

    fn assert_close(a: f32, b: f32) {
        let epsilon = 0.004;
        if (a - b).abs() > epsilon {
            panic!(format!("assert_close: {} too far from {}", a, b));
        }
    }

    mod voice_state {
        use super::*;

        impl VoiceState {
            fn get_phase(&self) -> f32 {
                match self {
                    VoiceState::Playing { phase, .. } => *phase,
                    VoiceState::Muted => panic!("get_phase: Muted"),
                }
            }
        }

        mod step {
            use super::*;

            #[test]
            fn reaches_2_pi_after_1_second() {
                let mut voice = VoiceState::default();
                voice.note_on(1.0);
                let sample_rate = 100;
                for _ in 0..(sample_rate - 1) {
                    voice.step(
                        sample_rate,
                        &Envelope {
                            attack: 0.0,
                            release: 0.0,
                        },
                    );
                }
                assert_close(
                    voice.get_phase(),
                    TAU * (sample_rate - 1) as f32 / sample_rate as f32,
                );
            }

            #[test]
            fn increases_the_phase_for_one_sample() {
                let mut voice = VoiceState::default();
                voice.note_on(1.0);
                assert_eq!(voice.get_phase(), 0.0);
                voice.step(
                    SAMPLE_RATE,
                    &Envelope {
                        attack: 0.0,
                        release: 0.0,
                    },
                );
                assert_eq!(voice.get_phase(), TAU / SAMPLE_RATE as f32);
            }

            #[test]
            fn wraps_around_at_2_pi() {
                let mut voice = VoiceState::default();
                voice.note_on(1.0);
                for _ in 0..SAMPLE_RATE {
                    voice.step(
                        SAMPLE_RATE,
                        &Envelope {
                            attack: 0.0,
                            release: 0.0,
                        },
                    );
                }
                assert_close(voice.get_phase(), 0.0);
            }
        }

        mod generators_handle_note_events {
            use super::*;

            #[test]
            fn switches_on_the_voice_with_the_same_index_as_the_note_event() {
                for i in 0..POLYPHONY {
                    let mut generators = sine_generators();
                    let voices = {
                        let mut result = [NoteEvent::NoteOff; POLYPHONY];
                        result[i] = NoteEvent::NoteOn { frequency: 42.0 };
                        result
                    };
                    generators.handle_note_events(voices);
                    let expected = {
                        let mut result = [VoiceState::Muted; POLYPHONY];
                        result[i] = VoiceState::Playing {
                            frequency: 42.0,
                            phase: 0.0,
                            envelope_phase: EnvelopePhase::Attacking {
                                attack_amplitude: 0.0,
                            },
                        };
                        result
                    };
                    assert_eq!(generators.voices, expected);
                }
            }

            #[test]
            fn switches_off_the_correct_voice() {
                for i in 0..POLYPHONY {
                    let mut generators = sine_generators();
                    let voices = {
                        let mut result = [NoteEvent::NoteOff; POLYPHONY];
                        result[i] = NoteEvent::NoteOn { frequency: 42.0 };
                        result
                    };
                    generators.handle_note_events(voices);
                    generators.handle_note_events([NoteEvent::NoteOff; POLYPHONY]);
                    generators.generate(SAMPLE_RATE, &mut [0.0]);
                    assert_eq!(generators.voices, [VoiceState::Muted; POLYPHONY]);
                }
            }
        }

        mod generators_generate {
            use super::*;

            fn buffer() -> [f32; 10] {
                [0.0; 10]
            }

            #[test]
            fn new_creates_as_many_voices_as_configured() {
                let generators = Generators::new(&cli::test::args(vec![]));
                assert_eq!(generators.voices.len(), POLYPHONY);
            }

            #[test]
            fn starts_at_zero() {
                let mut generators = monophonic_sine_generators();
                generators.voices[0].note_on(1.0);
                let buffer = &mut buffer();
                generators.generate(SAMPLE_RATE, buffer);
                assert_eq!(buffer[0], (TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn generates_sine_waves() {
                let mut generators = monophonic_sine_generators();
                generators.voices[0].note_on(1.0);
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], (TAU / SAMPLE_RATE as f32).sin());
                assert_eq!(buffer[1], (2.0 * TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn starts_with_phase_zero_after_pauses() {
                let mut generators = monophonic_sine_generators();
                generators.voices[0].note_on(1.0);
                generators.generate(SAMPLE_RATE, &mut buffer());
                generators.voices[0].note_off();
                generators.generate(SAMPLE_RATE, &mut buffer());
                generators.voices[0].note_on(1.0);
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], (TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn doesnt_reset_the_phase_when_changing_the_frequency_without_pause() {
                let mut generators = monophonic_sine_generators();
                generators.voices[0].note_on(1.0);
                generators.generate(SAMPLE_RATE, &mut buffer());
                generators.voices[0].note_on(1.1);
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert!(buffer[0] != 0.0, "{} should not equal {}", buffer[0], 0.0);
            }

            #[test]
            fn works_for_different_frequencies() {
                let mut generators = monophonic_sine_generators();
                generators.voices[0].note_on(300.0);
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], (300.0 * TAU / SAMPLE_RATE as f32).sin());
                assert_eq!(buffer[1], (2.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
                assert_eq!(buffer[8], (9.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn allows_to_change_the_frequency_later() {
                let mut generators = monophonic_sine_generators();
                generators.voices[0].note_on(300.0);
                generators.generate(SAMPLE_RATE, &mut buffer());
                generators.voices[0].note_on(500.0);
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(
                    buffer[0],
                    ((10.0 * 300.0 + 500.0) * TAU / SAMPLE_RATE as f32).sin()
                );
                assert_eq!(
                    buffer[1],
                    ((10.0 * 300.0 + 2.0 * 500.0) * TAU / SAMPLE_RATE as f32).sin()
                );
            }

            #[test]
            fn is_initially_muted() {
                let mut generators = Generators {
                    amplitude: 1.0,
                    midi_controller_volume: 1.0,
                    envelope: Envelope {
                        attack: 0.0,
                        release: 0.0,
                    },
                    wave_form: WaveForm::from_function(|x| x.sin(), 10000),
                    voices: vec![VoiceState::default()],
                };
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[1], 0.0);
                assert_eq!(buffer[2], 0.0);
            }

            #[test]
            fn can_be_muted() {
                let mut generators = monophonic_sine_generators();
                generators.voices[0].note_on(1.0);
                generators.generate(SAMPLE_RATE, &mut buffer());
                generators.voices[0].note_off();
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[1], 0.0);
                assert_eq!(buffer[2], 0.0);
            }

            #[test]
            fn allows_to_specify_the_wave_form() {
                let mut generators = Generators {
                    amplitude: 1.0,
                    midi_controller_volume: 1.0,
                    envelope: Envelope {
                        attack: 0.0,
                        release: 0.0,
                    },
                    wave_form: WaveForm::from_function(|phase| phase * 5.0, 10000),
                    voices: vec![VoiceState::default()],
                };
                generators.voices[0].note_on(1.0);
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_close(buffer[0], 5.0 * TAU / SAMPLE_RATE as f32);
                assert_close(buffer[1], 2.0 * 5.0 * TAU / SAMPLE_RATE as f32);
            }

            #[test]
            fn allows_to_scale_the_amplitude() {
                let mut generators = Generators {
                    amplitude: 0.25,
                    midi_controller_volume: 1.0,
                    envelope: Envelope {
                        attack: 0.0,
                        release: 0.0,
                    },
                    wave_form: WaveForm::from_function(|_phase| 0.4, 10000),
                    voices: vec![VoiceState::default()],
                };
                generators.voices[0].note_on(1.0);
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], 0.1);
            }

            #[test]
            fn allows_to_adjust_the_controller_volume_later() {
                let mut generators = Generators {
                    amplitude: 1.0,
                    midi_controller_volume: 1.0,
                    envelope: Envelope {
                        attack: 0.0,
                        release: 0.0,
                    },
                    wave_form: WaveForm::from_function(|_phase| 0.4, 10000),
                    voices: vec![VoiceState::default()],
                };
                generators.voices[0].note_on(1.0);
                generators.generate(SAMPLE_RATE, &mut buffer());
                generators.midi_controller_volume = 0.5;
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], 0.2);
            }

            mod envelope {
                use super::*;

                fn assert_elements_close(a: [f32; 10], b: [f32; 10]) {
                    let epsilon = 0.000001;
                    let mut close = true;
                    for (x, y) in a.iter().zip(b.iter()) {
                        if (x - y).abs() > epsilon {
                            println!("{} != {}", x, y);
                            close = false;
                        }
                    }
                    assert!(close, "not close enough: {:?} and {:?}", a, b);
                }

                #[test]
                fn allows_to_specify_an_attack_time() {
                    let mut generators = Generators {
                        amplitude: 1.0,
                        midi_controller_volume: 1.0,
                        envelope: Envelope {
                            attack: 0.5,
                            release: 0.0,
                        },
                        wave_form: WaveForm::from_function(|_phase| 0.5, 10000),
                        voices: vec![VoiceState::default()],
                    };
                    generators.voices[0].note_on(1.0);
                    let mut buffer = buffer();
                    generators.generate(10, &mut buffer);
                    assert_elements_close(
                        buffer,
                        [0.1, 0.2, 0.3, 0.4, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
                    );
                }

                #[test]
                fn does_not_reenter_an_attack_phase_for_subsequent_note_ons_when_playing() {
                    let mut generators = Generators {
                        amplitude: 1.0,
                        midi_controller_volume: 1.0,
                        envelope: Envelope {
                            attack: 0.5,
                            release: 0.0,
                        },
                        wave_form: WaveForm::from_function(|_phase| 0.5, 10000),
                        voices: vec![VoiceState::default()],
                    };
                    generators.voices[0].note_on(1.0);
                    generators.generate(10, &mut buffer());
                    generators.voices[0].note_on(1.0);
                    let mut second_buffer = buffer();
                    generators.generate(10, &mut second_buffer);
                    assert_elements_close(second_buffer, [0.5; 10]);
                }

                #[test]
                fn does_not_restart_an_attack_phase_for_subsequent_note_ons_while_in_attack_phase()
                {
                    let mut generators = Generators {
                        amplitude: 1.0,
                        midi_controller_volume: 1.0,
                        envelope: Envelope {
                            attack: 2.0,
                            release: 0.0,
                        },
                        wave_form: WaveForm::from_function(|_phase| 1.0, 10000),
                        voices: vec![VoiceState::default()],
                    };
                    generators.voices[0].note_on(1.0);
                    generators.generate(10, &mut buffer());
                    generators.voices[0].note_on(1.0);
                    let mut second_buffer = buffer();
                    generators.generate(10, &mut second_buffer);
                    let expected = {
                        let mut result = buffer();
                        for (i, cell) in result.iter_mut().enumerate() {
                            *cell = 0.5 + (i as f32 + 1.0) * 0.05;
                        }
                        result
                    };
                    assert_elements_close(second_buffer, expected);
                }

                #[test]
                fn allows_to_specify_a_release_time() {
                    let mut generators = Generators {
                        amplitude: 1.0,
                        midi_controller_volume: 1.0,
                        envelope: Envelope {
                            attack: 0.0,
                            release: 0.5,
                        },
                        wave_form: WaveForm::from_function(|_phase| 0.5, 10000),
                        voices: vec![VoiceState::default()],
                    };
                    generators.voices[0].note_on(1.0);
                    generators.generate(10, &mut buffer());
                    generators.voices[0].note_off();
                    let mut buffer = buffer();
                    generators.generate(10, &mut buffer);
                    assert_elements_close(
                        buffer,
                        [0.4, 0.3, 0.2, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
                    );
                }
            }

            mod polyphony {
                use super::*;

                #[test]
                fn does_not_overwrite_the_buffer_when_muted() {
                    let mut generators = monophonic_sine_generators();
                    generators.voices[0].note_off();
                    let mut buffer = buffer();
                    buffer[5] = 23.0;
                    generators.generate(SAMPLE_RATE, &mut buffer);
                    assert_eq!(buffer[5], 23.0);
                }

                #[test]
                fn adds_its_values_to_the_given_buffer() {
                    let mut generators = Generators {
                        amplitude: 0.5,
                        midi_controller_volume: 1.0,
                        envelope: Envelope {
                            attack: 0.0,
                            release: 0.0,
                        },
                        wave_form: WaveForm::from_function(|_phase| 0.5, 10000),
                        voices: vec![VoiceState::default()],
                    };
                    generators.voices[0].note_on(440.0);
                    let mut buffer = buffer();
                    buffer[0] = 0.1;
                    generators.generate(10, &mut buffer);
                    assert_eq!(buffer[0], 0.1 + 0.5 * 0.5);
                }

                #[test]
                fn adds_its_values_to_the_given_buffer_during_release() {
                    let sample_rate = 10;
                    let mut generators = Generators {
                        amplitude: 1.0,
                        midi_controller_volume: 1.0,
                        envelope: Envelope {
                            attack: 0.0,
                            release: 1.0,
                        },
                        wave_form: WaveForm::from_function(|_phase| 0.5, 10000),
                        voices: vec![VoiceState::default()],
                    };
                    let mut buffer = buffer();
                    generators.voices[0].note_on(440.0);
                    generators.generate(sample_rate, &mut buffer);
                    generators.voices[0].note_off();
                    buffer[0] = 0.1;
                    buffer[1] = 0.1;
                    generators.generate(sample_rate, &mut buffer);
                    assert_eq!(buffer[0], 0.1 + 0.5 * 0.9);
                    assert_close(buffer[1], 0.1 + 0.5 * 0.8);
                }
            }
        }
    }
}
