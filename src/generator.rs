const TAU: f32 = ::std::f32::consts::PI * 2.0;

pub enum OscillatorState {
    Playing { frequency: f32, phase: f32 },
    Muted,
}

pub struct Generator {
    amplitude: f32,
    wave_form: Box<Fn(f32) -> f32 + 'static + Send>,
    oscillator_state: OscillatorState,
}

impl Generator {
    pub fn new<F: Fn(f32) -> f32 + 'static + Send>(amplitude: f32, f: F) -> Generator {
        Generator {
            amplitude,
            wave_form: Box::new(f),
            oscillator_state: OscillatorState::Muted,
        }
    }

    pub fn set_frequency(&mut self, frequency: f32) {
        self.oscillator_state = OscillatorState::Playing {
            frequency,
            phase: match self.oscillator_state {
                OscillatorState::Playing { phase, .. } => phase,
                OscillatorState::Muted => 0.0,
            },
        };
    }

    pub fn mute(&mut self) {
        self.oscillator_state = OscillatorState::Muted;
    }

    fn crank_phase(&mut self, sample_rate: i32) {
        match self.oscillator_state {
            OscillatorState::Playing {
                frequency,
                ref mut phase,
            } => {
                *phase += frequency * TAU / sample_rate as f32;
                *phase %= TAU;
            }
            OscillatorState::Muted => {}
        }
    }

    pub fn generate(&mut self, sample_rate: i32, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            match self.oscillator_state {
                OscillatorState::Muted => {
                    *sample = 0.0;
                }
                OscillatorState::Playing { phase, .. } => {
                    *sample = (self.wave_form)(phase) * self.amplitude;
                }
            }
            self.crank_phase(sample_rate);
        }
    }
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
        let mut generator = Generator::new(1.0, |x| x.sin());
        generator.set_frequency(1.0);
        generator
    }

    impl OscillatorState {
        fn get_phase(&self) -> f32 {
            match self {
                OscillatorState::Playing { phase, .. } => *phase,
                OscillatorState::Muted => panic!("get_phase: Muted"),
            }
        }
    }

    mod crank_phase {
        use super::*;

        #[test]
        fn reaches_2_pi_after_1_second() {
            let mut generator = generator();
            let sample_rate = 100;
            for _ in 0..(sample_rate - 1) {
                generator.crank_phase(sample_rate);
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
            generator.crank_phase(SAMPLE_RATE);
            assert_eq!(
                generator.oscillator_state.get_phase(),
                TAU / SAMPLE_RATE as f32
            );
        }

        #[test]
        fn wraps_around_at_2_pi() {
            let mut generator = generator();
            for _ in 0..SAMPLE_RATE {
                generator.crank_phase(SAMPLE_RATE);
            }
            assert_close(generator.oscillator_state.get_phase(), 0.0);
        }
    }

    mod generator {
        use super::*;

        #[test]
        fn starts_at_zero() {
            let mut generator = generator();
            let buffer: &mut [f32] = &mut [42.0; 10];
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[0], 0.0);
        }

        #[test]
        fn generates_sine_waves() {
            let mut generator = generator();
            let buffer: &mut [f32] = &mut [42.0; 10];
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[1], (TAU / SAMPLE_RATE as f32).sin());
            assert_eq!(buffer[2], (2.0 * TAU / SAMPLE_RATE as f32).sin());
        }

        #[test]
        fn starts_with_phase_zero_after_pauses() {
            let buffer: &mut [f32] = &mut [42.0; 10];
            let mut generator = Generator::new(1.0, |x| x.sin());
            generator.set_frequency(1.0);
            generator.generate(SAMPLE_RATE, buffer);
            generator.mute();
            generator.generate(SAMPLE_RATE, buffer);
            generator.set_frequency(1.0);
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[0], 0.0);
        }

        #[test]
        fn doesnt_reset_the_phase_when_changing_the_frequency_without_pause() {
            let buffer: &mut [f32] = &mut [42.0; 10];
            let mut generator = Generator::new(1.0, |x| x.sin());
            generator.set_frequency(1.0);
            generator.generate(SAMPLE_RATE, buffer);
            generator.set_frequency(1.1);
            generator.generate(SAMPLE_RATE, buffer);
            assert!(buffer[0] != 0.0, "{} should not equal {}", buffer[0], 0.0);
        }

        #[test]
        fn works_for_different_frequencies() {
            let mut generator = generator();
            let buffer: &mut [f32] = &mut [42.0; 10];
            generator.set_frequency(300.0);
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[1], (300.0 * TAU / SAMPLE_RATE as f32).sin());
            assert_eq!(buffer[2], (2.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
            assert_eq!(buffer[9], (9.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
        }

        #[test]
        fn allows_to_change_the_frequency_later() {
            let mut generator = generator();
            let buffer: &mut [f32] = &mut [42.0; 10];
            generator.set_frequency(300.0);
            generator.generate(SAMPLE_RATE, buffer);
            generator.set_frequency(500.0);
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
            let mut generator = Generator::new(1.0, |x| x.sin());
            let buffer: &mut [f32] = &mut [42.0; 10];
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[1], 0.0);
            assert_eq!(buffer[2], 0.0);
        }

        #[test]
        fn can_be_muted() {
            let mut generator = generator();
            let buffer: &mut [f32] = &mut [42.0; 10];
            generator.set_frequency(1.0);
            generator.generate(SAMPLE_RATE, buffer);
            generator.mute();
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[1], 0.0);
            assert_eq!(buffer[2], 0.0);
        }

        #[test]
        fn allows_to_specify_the_wave_form() {
            let mut generator = Generator::new(1.0, |phase| phase * 5.0);
            generator.set_frequency(1.0);
            let buffer: &mut [f32] = &mut [42.0; 10];
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[0], 0.0);
            assert_close(buffer[1], 5.0 * TAU / SAMPLE_RATE as f32);
        }

        #[test]
        fn allows_to_scale_the_amplitude() {
            let buffer: &mut [f32] = &mut [42.0; 10];
            let mut generator = Generator::new(0.25, |_phase| 0.4);
            generator.set_frequency(1.0);
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[0], 0.1);
        }
    }
}

// fixme: pull out buffer?
