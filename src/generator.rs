const TAU: f32 = ::std::f32::consts::PI * 2.0;

pub struct Generator {
    pub frequency: Option<f32>,
    phase: f32,
    wave_form: Box<Fn(f32) -> f32 + 'static + Send>,
}

impl Generator {
    pub fn new<F: Fn(f32) -> f32 + 'static + Send>(f: F) -> Generator {
        Generator {
            frequency: None,
            phase: 0.0,
            wave_form: Box::new(f),
        }
    }

    fn crank_phase(&mut self, sample_rate: i32) {
        if let Some(frequency) = self.frequency {
            self.phase += frequency * TAU / sample_rate as f32;
            self.phase %= TAU;
        }
    }

    pub fn generate(&mut self, sample_rate: i32, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            match self.frequency {
                None => {
                    *sample = 0.0;
                }
                Some(_) => {
                    *sample = (self.wave_form)(self.phase);
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
        let mut generator = Generator::new(|x| x.sin());
        generator.frequency = Some(1.0);
        generator
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
                generator.phase,
                TAU * (sample_rate - 1) as f32 / sample_rate as f32,
            );
        }

        #[test]
        fn increases_the_phase_for_one_sample() {
            let mut generator = generator();
            assert_eq!(generator.phase, 0.0);
            generator.crank_phase(SAMPLE_RATE);
            assert_eq!(generator.phase, TAU / SAMPLE_RATE as f32);
        }

        #[test]
        fn wraps_around_at_2_pi() {
            let mut generator = generator();
            for _ in 0..SAMPLE_RATE {
                generator.crank_phase(SAMPLE_RATE);
            }
            assert_close(generator.phase, 0.0);
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
        fn works_for_different_frequencies() {
            let mut generator = generator();
            let buffer: &mut [f32] = &mut [42.0; 10];
            generator.frequency = Some(300.0);
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[1], (300.0 * TAU / SAMPLE_RATE as f32).sin());
            assert_eq!(buffer[2], (2.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
            assert_eq!(buffer[9], (9.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
        }

        #[test]
        fn allows_to_change_the_frequency_later() {
            let mut generator = generator();
            let buffer: &mut [f32] = &mut [42.0; 10];
            generator.frequency = Some(300.0);
            generator.generate(SAMPLE_RATE, buffer);
            generator.frequency = Some(500.0);
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
            let mut generator = Generator::new(|x| x.sin());
            let buffer: &mut [f32] = &mut [42.0; 10];
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[1], 0.0);
            assert_eq!(buffer[2], 0.0);
        }

        #[test]
        fn can_be_muted() {
            let mut generator = generator();
            let buffer: &mut [f32] = &mut [42.0; 10];
            generator.frequency = Some(1.0);
            generator.generate(SAMPLE_RATE, buffer);
            generator.frequency = None;
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[1], 0.0);
            assert_eq!(buffer[2], 0.0);
        }

        #[test]
        fn allows_to_specify_the_wave_form() {
            let mut generator = Generator::new(|phase| phase * 5.0);
            generator.frequency = Some(1.0);
            let buffer: &mut [f32] = &mut [42.0; 10];
            generator.generate(SAMPLE_RATE, buffer);
            assert_eq!(buffer[0], 0.0);
            assert_close(buffer[1], 5.0 * TAU / SAMPLE_RATE as f32);
        }
    }
}
