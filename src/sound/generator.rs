use evdev::{slot_map, Slots};
use sound::wave_form::WaveForm;
use sound::TAU;

#[derive(Clone)]
pub struct Args {
    pub amplitude: f32,
    pub release: f32,
    pub wave_form: WaveForm,
}

impl Args {
    pub fn unfold_generator_args(self) -> Slots<Args> {
        let mut args = self;
        args.amplitude /= 10.0;
        slot_map([0; 10], |_| args.clone())
    }
}

pub struct Generator {
    amplitude: f32,
    wave_form: WaveForm,
    release_per_sample: f32,
    oscillator_state: OscillatorState,
}

impl Generator {
    pub fn new(args: Args, sample_rate: i32) -> Generator {
        let Args {
            amplitude,
            release,
            wave_form,
        } = args;
        Generator {
            amplitude,
            wave_form,
            release_per_sample: 1.0 / (sample_rate as f32 * release),
            oscillator_state: OscillatorState::Muted,
        }
    }

    pub fn note_on(&mut self, frequency: f32) {
        self.oscillator_state = OscillatorState::Playing {
            frequency,
            phase: match self.oscillator_state {
                OscillatorState::Playing { phase, .. } => phase,
                OscillatorState::Releasing { .. } => 0.0,
                OscillatorState::Muted => 0.0,
            },
        };
    }

    pub fn note_off(&mut self) {
        match self.oscillator_state {
            OscillatorState::Playing { frequency, phase } => {
                self.oscillator_state = OscillatorState::Releasing {
                    release_amplitude: 1.0,
                    frequency,
                    phase,
                };
            }
            OscillatorState::Releasing { .. } => {}
            OscillatorState::Muted => {}
        }
    }

    fn crank_phase(&mut self, sample_rate: i32) {
        match self.oscillator_state {
            OscillatorState::Releasing {
                frequency,
                ref mut phase,
                ..
            }
            | OscillatorState::Playing {
                frequency,
                ref mut phase,
                ..
            } => {
                *phase += frequency * TAU / sample_rate as f32;
                *phase %= TAU;
            }
            OscillatorState::Muted => {}
        };
    }

    fn step_release(&mut self) {
        let mute = match self.oscillator_state {
            OscillatorState::Releasing {
                ref mut release_amplitude,
                ..
            } => {
                *release_amplitude -= self.release_per_sample;
                (*release_amplitude <= 0.0)
            }
            _ => false,
        };
        if mute {
            self.oscillator_state = OscillatorState::Muted;
        }
    }

    fn step(&mut self, sample_rate: i32) {
        self.crank_phase(sample_rate);
        self.step_release();
    }

    pub fn generate(&mut self, sample_rate: i32, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            self.step(sample_rate);
            match self.oscillator_state {
                OscillatorState::Playing { phase, .. } => {
                    *sample += self.wave_form.run(phase) * self.amplitude;
                }
                OscillatorState::Releasing {
                    phase,
                    release_amplitude,
                    ..
                } => {
                    *sample += self.wave_form.run(phase) * self.amplitude * release_amplitude;
                }
                OscillatorState::Muted => {}
            }
        }
    }
}

#[derive(Debug)]
pub enum OscillatorState {
    Playing {
        frequency: f32,
        phase: f32,
    },
    Releasing {
        frequency: f32,
        phase: f32,
        release_amplitude: f32,
    },
    Muted,
}

#[cfg(test)]
mod test {
    use super::*;

    mod args {
        use super::*;

        mod unfold_generator_args {
            use super::*;

            #[test]
            fn gives_every_slot_a_tenth_of_the_volume() {
                let args = Args {
                    amplitude: 1.0,
                    release: 0.0,
                    wave_form: WaveForm::new(|_| 0.0),
                };
                for slot_args in args.unfold_generator_args().into_iter() {
                    assert_eq!(slot_args.amplitude, 0.1);
                }
            }
        }
    }

    mod generator {
        use super::*;

        const SAMPLE_RATE: i32 = 44100;

        fn assert_close(a: f32, b: f32) {
            let epsilon = 0.004;
            if (a - b).abs() > epsilon {
                panic!(format!("assert_close: {} too far from {}", a, b));
            }
        }

        fn generator() -> Generator {
            let mut generator = Generator::new(
                Args {
                    amplitude: 1.0,
                    release: 0.0,
                    wave_form: WaveForm::new(|x| x.sin()),
                },
                SAMPLE_RATE,
            );
            generator.note_on(1.0);
            generator
        }

        impl OscillatorState {
            fn get_phase(&self) -> f32 {
                match self {
                    OscillatorState::Playing { phase, .. } => *phase,
                    OscillatorState::Releasing { phase, .. } => *phase,
                    OscillatorState::Muted => panic!("get_phase: Muted"),
                }
            }
        }

        mod step {
            use super::*;

            #[test]
            fn reaches_2_pi_after_1_second() {
                let mut generator = generator();
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
                let mut generator = generator();
                assert_eq!(generator.oscillator_state.get_phase(), 0.0);
                generator.step(SAMPLE_RATE);
                assert_eq!(
                    generator.oscillator_state.get_phase(),
                    TAU / SAMPLE_RATE as f32
                );
            }

            #[test]
            fn wraps_around_at_2_pi() {
                let mut generator = generator();
                for _ in 0..SAMPLE_RATE {
                    generator.step(SAMPLE_RATE);
                }
                assert_close(generator.oscillator_state.get_phase(), 0.0);
            }
        }

        mod generator {
            use super::*;

            fn buffer() -> [f32; 10] {
                [0.0; 10]
            }

            #[test]
            fn starts_at_zero() {
                let mut generator = generator();
                let buffer = &mut buffer();
                generator.generate(SAMPLE_RATE, buffer);
                assert_eq!(buffer[0], (TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn generates_sine_waves() {
                let mut generator = generator();
                let mut buffer = buffer();
                generator.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], (TAU / SAMPLE_RATE as f32).sin());
                assert_eq!(buffer[1], (2.0 * TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn starts_with_phase_zero_after_pauses() {
                let mut generator = generator();
                generator.note_on(1.0);
                generator.generate(SAMPLE_RATE, &mut buffer());
                generator.note_off();
                generator.generate(SAMPLE_RATE, &mut buffer());
                generator.note_on(1.0);
                let mut buffer = buffer();
                generator.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], (TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn doesnt_reset_the_phase_when_changing_the_frequency_without_pause() {
                let mut generator = generator();
                generator.note_on(1.0);
                generator.generate(SAMPLE_RATE, &mut buffer());
                generator.note_on(1.1);
                let mut buffer = buffer();
                generator.generate(SAMPLE_RATE, &mut buffer);
                assert!(buffer[0] != 0.0, "{} should not equal {}", buffer[0], 0.0);
            }

            #[test]
            fn works_for_different_frequencies() {
                let mut generator = generator();
                generator.note_on(300.0);
                let mut buffer = buffer();
                generator.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], (300.0 * TAU / SAMPLE_RATE as f32).sin());
                assert_eq!(buffer[1], (2.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
                assert_eq!(buffer[8], (9.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
            }

            #[test]
            fn allows_to_change_the_frequency_later() {
                let mut generator = generator();
                generator.note_on(300.0);
                generator.generate(SAMPLE_RATE, &mut buffer());
                generator.note_on(500.0);
                let mut buffer = buffer();
                generator.generate(SAMPLE_RATE, &mut buffer);
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
                let mut generator = Generator::new(
                    Args {
                        amplitude: 1.0,
                        release: 0.0,
                        wave_form: WaveForm::new(|x| x.sin()),
                    },
                    SAMPLE_RATE,
                );
                let mut buffer = buffer();
                generator.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[1], 0.0);
                assert_eq!(buffer[2], 0.0);
            }

            #[test]
            fn can_be_muted() {
                let mut generator = generator();
                generator.note_on(1.0);
                generator.generate(SAMPLE_RATE, &mut buffer());
                generator.note_off();
                let mut buffer = buffer();
                generator.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[1], 0.0);
                assert_eq!(buffer[2], 0.0);
            }

            #[test]
            fn allows_to_specify_the_wave_form() {
                let mut generator = Generator::new(
                    Args {
                        amplitude: 1.0,
                        release: 0.0,
                        wave_form: WaveForm::new(|phase| phase * 5.0),
                    },
                    SAMPLE_RATE,
                );
                generator.note_on(1.0);
                let mut buffer = buffer();
                generator.generate(SAMPLE_RATE, &mut buffer);
                assert_close(buffer[0], 5.0 * TAU / SAMPLE_RATE as f32);
                assert_close(buffer[1], 2.0 * 5.0 * TAU / SAMPLE_RATE as f32);
            }

            #[test]
            fn allows_to_scale_the_amplitude() {
                let mut generator = Generator::new(
                    Args {
                        amplitude: 0.25,
                        release: 0.0,
                        wave_form: WaveForm::new(|_phase| 0.4),
                    },
                    SAMPLE_RATE,
                );
                generator.note_on(1.0);
                let mut buffer = buffer();
                generator.generate(SAMPLE_RATE, &mut buffer);
                assert_eq!(buffer[0], 0.1);
            }

            #[test]
            fn allows_to_specify_a_release_time() {
                let mut generator = Generator::new(
                    Args {
                        amplitude: 1.0,
                        release: 0.5,
                        wave_form: WaveForm::new(|_phase| 0.5),
                    },
                    10,
                );
                generator.note_on(1.0);
                generator.generate(10, &mut buffer());
                generator.note_off();
                let mut buffer = buffer();
                generator.generate(10, &mut buffer);
                let expected: [f32; 10] = [0.4, 0.3, 0.2, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
                let epsilon = 0.0000001;
                let mut close = true;
                for (a, b) in buffer.iter().zip(expected.iter()) {
                    if (a - b).abs() > epsilon {
                        close = false;
                    }
                }
                assert!(close, "not close enough: {:?} and {:?}", buffer, expected);
            }

            mod polyphony {
                use super::*;

                #[test]
                fn does_not_overwrite_the_buffer_when_muted() {
                    let mut generator = generator();
                    generator.note_off();
                    let mut buffer = buffer();
                    buffer[5] = 23.0;
                    generator.generate(SAMPLE_RATE, &mut buffer);
                    assert_eq!(buffer[5], 23.0);
                }

                #[test]
                fn adds_its_values_to_the_given_buffer() {
                    let sample_rate = 10;
                    let mut generator = Generator::new(
                        Args {
                            amplitude: 0.5,
                            release: 0.0,
                            wave_form: WaveForm::new(|_phase| 0.5),
                        },
                        sample_rate,
                    );
                    generator.note_on(440.0);
                    let mut buffer = buffer();
                    buffer[0] = 0.1;
                    generator.generate(sample_rate, &mut buffer);
                    assert_eq!(buffer[0], 0.1 + 0.5 * 0.5);
                }

                #[test]
                fn adds_its_values_to_the_given_buffer_during_release() {
                    let sample_rate = 10;
                    let mut a = Generator::new(
                        Args {
                            amplitude: 1.0,
                            release: 1.0,
                            wave_form: WaveForm::new(|_phase| 0.5),
                        },
                        sample_rate,
                    );
                    a.note_on(440.0);
                    a.note_off();
                    let mut buffer = buffer();
                    buffer[0] = 0.1;
                    buffer[1] = 0.1;
                    a.generate(sample_rate, &mut buffer);
                    assert_eq!(buffer[0], 0.1 + 0.5 * 0.9);
                    assert_close(buffer[1], 0.1 + 0.5 * 0.8);
                }
            }
        }
    }
}
