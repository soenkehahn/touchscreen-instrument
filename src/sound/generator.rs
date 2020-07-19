use crate::cli;
use crate::sound::wave_form::WaveForm;
use crate::sound::TAU;
use crate::utils::Slots;

pub struct Generators {
    amplitude: f32,
    pub midi_controller_volume: f32,
    pub wave_form: WaveForm,
    pub slots: Vec<Generator>,
}

impl Generators {
    pub fn new(sample_rate: i32, cli_args: &cli::Args) -> Generators {
        let unit_slots: Slots<()> = [(); 10];
        let slots = unit_slots.len();
        Generators {
            amplitude: cli_args.volume / slots as f32,
            midi_controller_volume: 1.0,
            wave_form: WaveForm::new(&cli_args.wave_form_config),
            slots: vec![Generator::new(sample_rate, 0.005, 0.005); slots],
        }
    }

    pub fn generate(&mut self, sample_rate: i32, buffer: &mut [f32]) {
        for generator in self.slots.iter_mut() {
            for sample in buffer.iter_mut() {
                generator.step(sample_rate);
                match generator.oscillator_state {
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

#[derive(Debug, Clone)]
pub struct Generator {
    attack_per_sample: f32,
    release_per_sample: f32,
    oscillator_state: VoiceState,
}

impl Generator {
    pub fn new(sample_rate: i32, attack: f32, release: f32) -> Generator {
        Generator {
            attack_per_sample: 1.0 / (sample_rate as f32 * attack),
            release_per_sample: 1.0 / (sample_rate as f32 * release),
            oscillator_state: VoiceState::Muted,
        }
    }

    pub fn note_on(&mut self, frequency: f32) {
        self.oscillator_state = match self.oscillator_state {
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
        self.oscillator_state = match self.oscillator_state {
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

    fn crank_phase(&mut self, sample_rate: i32) {
        match self.oscillator_state {
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

    fn step_envelope(&mut self) {
        let next = match self.oscillator_state {
            VoiceState::Playing {
                frequency,
                phase,
                ref mut envelope_phase,
            } => match envelope_phase {
                EnvelopePhase::Attacking {
                    ref mut attack_amplitude,
                } => {
                    *attack_amplitude += self.attack_per_sample;
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
                    *release_amplitude -= self.release_per_sample;
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
            self.oscillator_state = n;
        }
    }

    fn step(&mut self, sample_rate: i32) {
        self.crank_phase(sample_rate);
        self.step_envelope();
    }
}

#[derive(Debug, Clone)]
pub enum VoiceState {
    Playing {
        frequency: f32,
        phase: f32,
        envelope_phase: EnvelopePhase,
    },
    Muted,
}

#[derive(Debug, Clone)]
pub enum EnvelopePhase {
    Attacking { attack_amplitude: f32 },
    FullVolume,
    Releasing { release_amplitude: f32 },
}

#[cfg(test)]
pub mod test {
    use super::*;

    const SAMPLE_RATE: i32 = 44100;

    pub fn sine_generators() -> Generators {
        Generators {
            amplitude: 1.0,
            midi_controller_volume: 1.0,
            wave_form: WaveForm::from_function(|x| x.sin(), SAMPLE_RATE as usize),
            slots: vec![sine_generator(); 10],
        }
    }

    fn monophonic_sine_generators() -> Generators {
        Generators {
            amplitude: 1.0,
            midi_controller_volume: 1.0,
            wave_form: WaveForm::from_function(|x| x.sin(), SAMPLE_RATE as usize),
            slots: vec![sine_generator()],
        }
    }

    fn sine_generator() -> Generator {
        let mut generator = Generator::new(SAMPLE_RATE, 0.0, 0.0);
        generator.note_on(1.0);
        generator
    }

    mod generator {
        use super::*;

        fn assert_close(a: f32, b: f32) {
            let epsilon = 0.004;
            if (a - b).abs() > epsilon {
                panic!(format!("assert_close: {} too far from {}", a, b));
            }
        }

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
                let mut generator = sine_generator();
                let sample_rate = 100;
                for _ in 0..(sample_rate - 1) {
                    generator.step(sample_rate);
                }
                assert_close(
                    generator.oscillator_state.get_phase(),
                    TAU * (sample_rate - 1) as f32 / sample_rate as f32,
                );
            }

            #[test]
            fn increases_the_phase_for_one_sample() {
                let mut generator = sine_generator();
                assert_eq!(generator.oscillator_state.get_phase(), 0.0);
                generator.step(SAMPLE_RATE);
                assert_eq!(
                    generator.oscillator_state.get_phase(),
                    TAU / SAMPLE_RATE as f32
                );
            }

            #[test]
            fn wraps_around_at_2_pi() {
                let mut generator = sine_generator();
                for _ in 0..SAMPLE_RATE {
                    generator.step(SAMPLE_RATE);
                }
                assert_close(generator.oscillator_state.get_phase(), 0.0);
            }
        }

        mod generators {
            use super::*;
            use crate::utils::Slots;

            fn buffer() -> [f32; 10] {
                [0.0; 10]
            }

            #[test]
            fn new_creates_as_many_voices_as_there_are_slots() {
                let generators = Generators::new(SAMPLE_RATE, &cli::test::args(vec![]));
                let slots: Slots<()> = [(); 10];
                assert_eq!(generators.slots.len(), slots.len());
            }

            #[test]
            fn starts_at_zero() {
                let mut generators = monophonic_sine_generators();
                let buffer = &mut buffer();
                generators.generate(SAMPLE_RATE, buffer);
                assert_eq!(buffer[0], (TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn generates_sine_waves() {
                let mut generator = monophonic_sine_generators();
                let mut buffer = buffer();
                generator.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], (TAU / SAMPLE_RATE as f32).sin());
                assert_eq!(buffer[1], (2.0 * TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn starts_with_phase_zero_after_pauses() {
                let mut generators = monophonic_sine_generators();
                generators.slots[0].note_on(1.0);
                generators.generate(SAMPLE_RATE, &mut buffer());
                generators.slots[0].note_off();
                generators.generate(SAMPLE_RATE, &mut buffer());
                generators.slots[0].note_on(1.0);
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], (TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn doesnt_reset_the_phase_when_changing_the_frequency_without_pause() {
                let mut generators = monophonic_sine_generators();
                generators.slots[0].note_on(1.0);
                generators.generate(SAMPLE_RATE, &mut buffer());
                generators.slots[0].note_on(1.1);
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert!(buffer[0] != 0.0, "{} should not equal {}", buffer[0], 0.0);
            }

            #[test]
            fn works_for_different_frequencies() {
                let mut generators = monophonic_sine_generators();
                generators.slots[0].note_on(300.0);
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], (300.0 * TAU / SAMPLE_RATE as f32).sin());
                assert_eq!(buffer[1], (2.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
                assert_eq!(buffer[8], (9.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn allows_to_change_the_frequency_later() {
                let mut generators = monophonic_sine_generators();
                generators.slots[0].note_on(300.0);
                generators.generate(SAMPLE_RATE, &mut buffer());
                generators.slots[0].note_on(500.0);
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
                    wave_form: WaveForm::from_function(|x| x.sin(), 10000),
                    slots: vec![Generator::new(SAMPLE_RATE, 0.0, 0.0)],
                };
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[1], 0.0);
                assert_eq!(buffer[2], 0.0);
            }

            #[test]
            fn can_be_muted() {
                let mut generators = monophonic_sine_generators();
                generators.slots[0].note_on(1.0);
                generators.generate(SAMPLE_RATE, &mut buffer());
                generators.slots[0].note_off();
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
                    wave_form: WaveForm::from_function(|phase| phase * 5.0, 10000),
                    slots: vec![Generator::new(SAMPLE_RATE, 0.0, 0.0)],
                };
                generators.slots[0].note_on(1.0);
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
                    wave_form: WaveForm::from_function(|_phase| 0.4, 10000),
                    slots: vec![Generator::new(SAMPLE_RATE, 0.0, 0.0)],
                };
                generators.slots[0].note_on(1.0);
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], 0.1);
            }

            #[test]
            fn allows_to_adjust_the_controller_volume_later() {
                let mut generators = Generators {
                    amplitude: 1.0,
                    midi_controller_volume: 1.0,
                    wave_form: WaveForm::from_function(|_phase| 0.4, 10000),
                    slots: vec![Generator::new(SAMPLE_RATE, 0.0, 0.0)],
                };
                generators.slots[0].note_on(1.0);
                generators.generate(SAMPLE_RATE, &mut buffer());
                generators.midi_controller_volume = 0.5;
                let mut buffer = buffer();
                generators.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], 0.2);
            }

            mod adsr {
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
                        wave_form: WaveForm::from_function(|_phase| 0.5, 10000),
                        slots: vec![Generator::new(10, 0.5, 0.0)],
                    };
                    generators.slots[0].note_on(1.0);
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
                        wave_form: WaveForm::from_function(|_phase| 0.5, 10000),
                        slots: vec![Generator::new(10, 0.5, 0.0)],
                    };
                    generators.slots[0].note_on(1.0);
                    generators.generate(10, &mut buffer());
                    generators.slots[0].note_on(1.0);
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
                        wave_form: WaveForm::from_function(|_phase| 1.0, 10000),
                        slots: vec![Generator::new(10, 2.0, 0.0)],
                    };
                    generators.slots[0].note_on(1.0);
                    generators.generate(10, &mut buffer());
                    generators.slots[0].note_on(1.0);
                    let mut second_buffer = buffer();
                    generators.generate(10, &mut second_buffer);
                    let mut expected = buffer();
                    for i in 0..10 {
                        expected[i] = 0.5 + (i as f32 + 1.0) * 0.05;
                    }
                    assert_elements_close(second_buffer, expected);
                }

                #[test]
                fn allows_to_specify_a_release_time() {
                    let mut generators = Generators {
                        amplitude: 1.0,
                        midi_controller_volume: 1.0,
                        wave_form: WaveForm::from_function(|_phase| 0.5, 10000),
                        slots: vec![Generator::new(10, 0.0, 0.5)],
                    };
                    generators.slots[0].note_on(1.0);
                    generators.generate(10, &mut buffer());
                    generators.slots[0].note_off();
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
                    generators.slots[0].note_off();
                    let mut buffer = buffer();
                    buffer[5] = 23.0;
                    generators.generate(SAMPLE_RATE, &mut buffer);
                    assert_eq!(buffer[5], 23.0);
                }

                #[test]
                fn adds_its_values_to_the_given_buffer() {
                    let sample_rate = 10;
                    let mut generators = Generators {
                        amplitude: 0.5,
                        midi_controller_volume: 1.0,
                        wave_form: WaveForm::from_function(|_phase| 0.5, 10000),
                        slots: vec![Generator::new(sample_rate, 0.0, 0.0)],
                    };
                    generators.slots[0].note_on(440.0);
                    let mut buffer = buffer();
                    buffer[0] = 0.1;
                    generators.generate(sample_rate, &mut buffer);
                    assert_eq!(buffer[0], 0.1 + 0.5 * 0.5);
                }

                #[test]
                fn adds_its_values_to_the_given_buffer_during_release() {
                    let sample_rate = 10;
                    let mut generators = Generators {
                        amplitude: 1.0,
                        midi_controller_volume: 1.0,
                        wave_form: WaveForm::from_function(|_phase| 0.5, 10000),
                        slots: vec![Generator::new(sample_rate, 0.0, 1.0)],
                    };
                    let mut buffer = buffer();
                    generators.slots[0].note_on(440.0);
                    generators.generate(sample_rate, &mut buffer);
                    generators.slots[0].note_off();
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
