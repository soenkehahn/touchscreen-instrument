const TAU: f32 = ::std::f32::consts::PI * 2.0;

pub struct Args<F: Fn(f32) -> f32 + 'static + Send> {
    pub amplitude: f32,
    pub decay: f32,
    pub wave_form: F,
}

pub struct Generator {
    amplitude: f32,
    wave_form: Box<Fn(f32) -> f32 + 'static + Send>,
    decay_per_sample: f32,
    oscillator_state: OscillatorState,
}

impl Generator {
    pub fn new<F: Fn(f32) -> f32 + 'static + Send>(args: Args<F>, sample_rate: i32) -> Generator {
        let Args {
            amplitude,
            decay,
            wave_form,
        } = args;
        Generator {
            amplitude,
            wave_form: Box::new(wave_form),
            decay_per_sample: 1.0 / (sample_rate as f32 * decay),
            oscillator_state: OscillatorState::Muted,
        }
    }

    pub fn note_on(&mut self, frequency: f32) {
        self.oscillator_state = OscillatorState::Playing {
            frequency,
            phase: match self.oscillator_state {
                OscillatorState::Playing { phase, .. } => phase,
                OscillatorState::Decaying { .. } => 0.0,
                OscillatorState::Muted => 0.0,
            },
        };
    }

    pub fn note_off(&mut self) {
        match self.oscillator_state {
            OscillatorState::Playing { frequency, phase } => {
                self.oscillator_state = OscillatorState::Decaying {
                    decay_amplitude: 1.0,
                    frequency,
                    phase,
                };
            }
            OscillatorState::Decaying { .. } => {}
            OscillatorState::Muted => {}
        }
    }

    fn crank_phase(&mut self, sample_rate: i32) {
        match self.oscillator_state {
            OscillatorState::Decaying {
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

    fn step_decay(&mut self) {
        let mute = match self.oscillator_state {
            OscillatorState::Decaying {
                ref mut decay_amplitude,
                ..
            } => {
                *decay_amplitude -= self.decay_per_sample;
                (*decay_amplitude <= 0.0)
            }
            _ => false,
        };
        if mute {
            self.oscillator_state = OscillatorState::Muted;
        }
    }

    fn step(&mut self, sample_rate: i32) {
        self.crank_phase(sample_rate);
        self.step_decay();
    }

    pub fn generate(&mut self, sample_rate: i32, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            match self.oscillator_state {
                OscillatorState::Playing { phase, .. } => {
                    *sample = (self.wave_form)(phase) * self.amplitude;
                }
                OscillatorState::Decaying {
                    phase,
                    decay_amplitude,
                    ..
                } => {
                    *sample = (self.wave_form)(phase) * self.amplitude * decay_amplitude;
                }
                OscillatorState::Muted => {
                    *sample = 0.0;
                }
            }
            self.step(sample_rate);
        }
    }
}

#[derive(Debug)]
pub enum OscillatorState {
    Playing {
        frequency: f32,
        phase: f32,
    },
    Decaying {
        frequency: f32,
        phase: f32,
        decay_amplitude: f32,
    },
    Muted,
}

#[cfg(test)]
mod test {
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
                decay: 0.0,
                wave_form: |x| x.sin(),
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
                OscillatorState::Decaying { phase, .. } => *phase,
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

        #[test]
        fn starts_at_zero() {
            let buffer: &mut [f32] = &mut [42.0; 10];
            let mut generator = generator();
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[0], 0.0);
        }

        #[test]
        fn generates_sine_waves() {
            let buffer: &mut [f32] = &mut [42.0; 10];
            let mut generator = generator();
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[1], (TAU / SAMPLE_RATE as f32).sin());
            assert_eq!(buffer[2], (2.0 * TAU / SAMPLE_RATE as f32).sin());
        }

        #[test]
        fn starts_with_phase_zero_after_pauses() {
            let buffer: &mut [f32] = &mut [42.0; 10];
            let mut generator = generator();
            generator.note_on(1.0);
            generator.generate(SAMPLE_RATE, buffer);
            generator.note_off();
            generator.generate(SAMPLE_RATE, buffer);
            generator.note_on(1.0);
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[0], 0.0);
        }

        #[test]
        fn doesnt_reset_the_phase_when_changing_the_frequency_without_pause() {
            let buffer: &mut [f32] = &mut [42.0; 10];
            let mut generator = generator();
            generator.note_on(1.0);
            generator.generate(SAMPLE_RATE, buffer);
            generator.note_on(1.1);
            generator.generate(SAMPLE_RATE, buffer);
            assert!(buffer[0] != 0.0, "{} should not equal {}", buffer[0], 0.0);
        }

        #[test]
        fn works_for_different_frequencies() {
            let buffer: &mut [f32] = &mut [42.0; 10];
            let mut generator = generator();
            generator.note_on(300.0);
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[1], (300.0 * TAU / SAMPLE_RATE as f32).sin());
            assert_eq!(buffer[2], (2.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
            assert_eq!(buffer[9], (9.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
        }

        #[test]
        fn allows_to_change_the_frequency_later() {
            let buffer: &mut [f32] = &mut [42.0; 10];
            let mut generator = generator();
            generator.note_on(300.0);
            generator.generate(SAMPLE_RATE, buffer);
            generator.note_on(500.0);
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[0], ((10.0 * 300.0) * TAU / SAMPLE_RATE as f32).sin());
            assert_eq!(
                buffer[1],
                ((10.0 * 300.0 + 500.0) * TAU / SAMPLE_RATE as f32).sin()
            );
            assert_eq!(
                buffer[2],
                ((10.0 * 300.0 + 2.0 * 500.0) * TAU / SAMPLE_RATE as f32).sin()
            );
        }

        #[test]
        fn is_initially_muted() {
            let buffer: &mut [f32] = &mut [42.0; 10];
            let mut generator = Generator::new(
                Args {
                    amplitude: 1.0,
                    decay: 0.0,
                    wave_form: |x| x.sin(),
                },
                SAMPLE_RATE,
            );
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[1], 0.0);
            assert_eq!(buffer[2], 0.0);
        }

        #[test]
        fn can_be_muted() {
            let buffer: &mut [f32] = &mut [42.0; 10];
            let mut generator = generator();
            generator.note_on(1.0);
            generator.generate(SAMPLE_RATE, buffer);
            generator.note_off();
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[1], 0.0);
            assert_eq!(buffer[2], 0.0);
        }

        #[test]
        fn allows_to_specify_the_wave_form() {
            let buffer: &mut [f32] = &mut [42.0; 10];
            let mut generator = Generator::new(
                Args {
                    amplitude: 1.0,
                    decay: 0.0,
                    wave_form: |phase| phase * 5.0,
                },
                SAMPLE_RATE,
            );
            generator.note_on(1.0);
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[0], 0.0);
            assert_close(buffer[1], 5.0 * TAU / SAMPLE_RATE as f32);
        }

        #[test]
        fn allows_to_scale_the_amplitude() {
            let buffer: &mut [f32] = &mut [42.0; 10];
            let mut generator = Generator::new(
                Args {
                    amplitude: 0.25,
                    decay: 0.0,
                    wave_form: |_phase| 0.4,
                },
                SAMPLE_RATE,
            );
            generator.note_on(1.0);
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[0], 0.1);
        }

        #[test]
        fn allows_to_specify_a_decay_time() {
            let buffer: &mut [f32] = &mut [42.0; 10];
            let mut generator = Generator::new(
                Args {
                    amplitude: 1.0,
                    decay: 0.5,
                    wave_form: |_phase| 0.5,
                },
                10,
            );
            generator.note_on(1.0);
            generator.generate(10, buffer);
            generator.note_off();
            generator.generate(10, buffer);
            let expected = [0.5, 0.4, 0.3, 0.2, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0];
            let epsilon = 0.0000001;
            let mut close = true;
            for (a, b) in buffer.iter().zip(expected.iter()) {
                if (a - b).abs() > epsilon {
                    close = false;
                }
            }
            assert!(close, "not close enough: {:?} and {:?}", buffer, expected);
        }
    }
}
