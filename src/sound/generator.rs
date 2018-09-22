use evdev::{slot_map, Slots};
use sound::wave_form::WaveForm;
use sound::TAU;

#[derive(Clone)]
pub struct Args {
    pub amplitude: f32,
    pub attack: f32,
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

#[derive(Debug)]
pub struct Generator {
    amplitude: f32,
    wave_form: WaveForm,
    attack_per_sample: f32,
    release_per_sample: f32,
    oscillator_state: OscillatorState,
}

impl Generator {
    pub fn new(args: Args, sample_rate: i32) -> Generator {
        let Args {
            amplitude,
            attack,
            release,
            wave_form,
        } = args;
        Generator {
            amplitude,
            wave_form,
            attack_per_sample: 1.0 / (sample_rate as f32 * attack),
            release_per_sample: 1.0 / (sample_rate as f32 * release),
            oscillator_state: OscillatorState::Muted,
        }
    }

    pub fn note_on(&mut self, frequency: f32) {
        self.oscillator_state = match self.oscillator_state {
            OscillatorState::Playing { phase, .. } => OscillatorState::Playing { frequency, phase },
            OscillatorState::Attacking {
                phase,
                attack_amplitude,
                ..
            } => OscillatorState::Attacking {
                frequency,
                phase,
                attack_amplitude,
            },
            OscillatorState::Releasing { .. } => OscillatorState::Attacking {
                frequency,
                phase: 0.0,
                attack_amplitude: 0.0,
            },
            OscillatorState::Muted => OscillatorState::Attacking {
                frequency,
                phase: 0.0,
                attack_amplitude: 0.0,
            },
        }
    }

    pub fn note_off(&mut self) {
        match self.oscillator_state {
            OscillatorState::Playing { frequency, phase } => {
                self.oscillator_state = OscillatorState::Releasing {
                    frequency,
                    phase,
                    release_amplitude: 1.0,
                };
            }
            OscillatorState::Attacking {
                frequency,
                phase,
                attack_amplitude,
            } => {
                self.oscillator_state = OscillatorState::Releasing {
                    frequency,
                    phase,
                    release_amplitude: attack_amplitude,
                }
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
            }
            | OscillatorState::Attacking {
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
        let next = match self.oscillator_state {
            OscillatorState::Releasing {
                ref mut release_amplitude,
                ..
            } => {
                *release_amplitude -= self.release_per_sample;
                if *release_amplitude <= 0.0 {
                    Some(OscillatorState::Muted)
                } else {
                    None
                }
            }
            OscillatorState::Attacking {
                frequency,
                phase,
                ref mut attack_amplitude,
            } => {
                *attack_amplitude += self.attack_per_sample;
                if *attack_amplitude >= 1.0 {
                    Some(OscillatorState::Playing { frequency, phase })
                } else {
                    None
                }
            }
            _ => None,
        };
        match next {
            None => {}
            Some(n) => {
                self.oscillator_state = n;
            }
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
                    release_amplitude: adsr_amplitude,
                    ..
                }
                | OscillatorState::Attacking {
                    phase,
                    attack_amplitude: adsr_amplitude,
                    ..
                } => {
                    *sample += self.wave_form.run(phase) * self.amplitude * adsr_amplitude;
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
    Attacking {
        frequency: f32,
        phase: f32,
        attack_amplitude: f32,
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
                    attack: 0.0,
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
                    attack: 0.0,
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
                    OscillatorState::Attacking { phase, .. } => *phase,
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
                        attack: 0.0,
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
                        attack: 0.0,
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
                        attack: 0.0,
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
                    let mut generator = Generator::new(
                        Args {
                            attack: 0.5,
                            amplitude: 1.0,
                            release: 0.0,
                            wave_form: WaveForm::new(|_phase| 0.5),
                        },
                        10,
                    );
                    generator.note_on(1.0);
                    let mut buffer = buffer();
                    generator.generate(10, &mut buffer);
                    assert_elements_close(
                        buffer,
                        [0.1, 0.2, 0.3, 0.4, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5],
                    );
                }

                #[test]
                fn does_not_reenter_an_attack_phase_for_subsequent_note_ons_when_playing() {
                    let mut generator = Generator::new(
                        Args {
                            attack: 0.5,
                            amplitude: 1.0,
                            release: 0.0,
                            wave_form: WaveForm::new(|_phase| 0.5),
                        },
                        10,
                    );
                    generator.note_on(1.0);
                    generator.generate(10, &mut buffer());
                    generator.note_on(1.0);
                    let mut second_buffer = buffer();
                    generator.generate(10, &mut second_buffer);
                    assert_elements_close(second_buffer, [0.5; 10]);
                }

                #[test]
                fn does_not_restart_an_attack_phase_for_subsequent_note_ons_while_in_attack_phase()
                {
                    let mut generator = Generator::new(
                        Args {
                            attack: 2.0,
                            amplitude: 1.0,
                            release: 0.0,
                            wave_form: WaveForm::new(|_phase| 1.0),
                        },
                        10,
                    );
                    generator.note_on(1.0);
                    generator.generate(10, &mut buffer());
                    generator.note_on(1.0);
                    let mut second_buffer = buffer();
                    generator.generate(10, &mut second_buffer);
                    let mut expected = buffer();
                    for i in 0..10 {
                        expected[i] = 0.5 + (i as f32 + 1.0) * 0.05;
                    }
                    assert_elements_close(second_buffer, expected);
                }

                #[test]
                fn allows_to_specify_a_release_time() {
                    let mut generator = Generator::new(
                        Args {
                            amplitude: 1.0,
                            attack: 0.0,
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
                            attack: 0.0,
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
                            attack: 0.0,
                            release: 1.0,
                            wave_form: WaveForm::new(|_phase| 0.5),
                        },
                        sample_rate,
                    );
                    let mut buffer = buffer();
                    a.note_on(440.0);
                    a.generate(sample_rate, &mut buffer);
                    a.note_off();
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
