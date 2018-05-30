const TAU: f32 = ::std::f32::consts::PI * 2.0;

pub struct Generator {
    muted: bool,
    pub frequency: f32,
    phase: f32,
}

impl Generator {
    pub fn new(frequency: f32) -> Generator {
        Generator {
            muted: true,
            frequency: frequency,
            phase: 0.0,
        }
    }

    fn crank_phase(&mut self, sample_rate: i32) {
        self.phase += self.frequency * TAU / sample_rate as f32;
        while self.phase >= TAU {
            self.phase -= TAU
        }
    }

    pub fn generate(&mut self, sample_rate: i32, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            if self.muted {
                *sample = 0.0;
            } else {
                *sample = self.phase.sin();
            }
            self.crank_phase(sample_rate);
        }
    }
}

test_suite! {
    use super::*;

    const SAMPLE_RATE : i32 = 44100;

    fn assert_close(a: f32, b: f32) {
        let epsilon = 0.004;
        if (a - b).abs() > epsilon {
            panic!(format!("assert_close: {} too far from {}", a, b));
        }
    }

    fixture generator() -> Generator {
        setup(&mut self) {
            let mut generator = Generator::new(1.0);
            generator.muted = false;
            generator
        }
    }

    test crank_phase_reaches_2_pi_after_1_second(generator) {
        let sample_rate = 100;
        for _ in 0..(sample_rate - 1) {
            generator.val.crank_phase(sample_rate);
        }
        assert_close(
            generator.val.phase,
            TAU * (sample_rate - 1) as f32 / sample_rate as f32
        );
    }

    test crank_phase_increases_the_phase_for_one_sample(generator) {
        assert_eq!(generator.val.phase, 0.0);
        generator.val.crank_phase(SAMPLE_RATE);
        assert_eq!(generator.val.phase, TAU / SAMPLE_RATE as f32);
    }

    test crank_phase_wraps_around_at_2_pi(generator) {
        for _ in 0..SAMPLE_RATE {
            generator.val.crank_phase(SAMPLE_RATE);
        }
        assert_close(generator.val.phase, 0.0);
    }

    test it_starts_at_zero(generator) {
        let buffer: &mut [f32] = &mut [42.0; 10];
        generator.val.generate(SAMPLE_RATE, buffer);
        assert_eq!(buffer[0], 0.0);
    }

    test it_generates_sine_waves(generator) {
        let buffer: &mut [f32] = &mut [42.0; 10];
        generator.val.generate(SAMPLE_RATE, buffer);
        assert_eq!(buffer[1], (TAU / SAMPLE_RATE as f32).sin());
        assert_eq!(buffer[2], (2.0 * TAU / SAMPLE_RATE as f32).sin());
    }

    test it_works_for_different_frequencies(generator) {
        let buffer: &mut [f32] = &mut [42.0; 10];
        generator.val.frequency = 300.0;
        generator.val.generate(SAMPLE_RATE, buffer);
        assert_eq!(buffer[1], (300.0 * TAU / SAMPLE_RATE as f32).sin());
        assert_eq!(buffer[2], (2.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
        assert_eq!(buffer[9], (9.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
    }

    test it_allows_to_change_the_frequency_later(generator) {
        let buffer: &mut [f32] = &mut [42.0; 10];
        generator.val.frequency = 300.0;
        generator.val.generate(SAMPLE_RATE, buffer);
        generator.val.frequency = 500.0;
        generator.val.generate(SAMPLE_RATE, buffer);
        assert_eq!(buffer[0], ((10.0 * 300.0) * TAU / SAMPLE_RATE as f32).sin());
        assert_eq!(buffer[1], ((10.0 * 300.0 + 500.0) * TAU / SAMPLE_RATE as f32).sin());
        assert_eq!(buffer[2], ((10.0 * 300.0 + 2.0 * 500.0) * TAU / SAMPLE_RATE as f32).sin());
    }

    test it_is_initially_muted() {
        let mut generator = Generator::new(1.0);
        let buffer: &mut [f32] = &mut [42.0; 10];
        generator.generate(SAMPLE_RATE, buffer);
        assert_eq!(buffer[1], 0.0);
        assert_eq!(buffer[2], 0.0);
    }

    test it_can_be_muted(generator) {
        let buffer: &mut [f32] = &mut [42.0; 10];
        generator.val.muted = false;
        generator.val.generate(SAMPLE_RATE, buffer);
        generator.val.muted = true;
        generator.val.generate(SAMPLE_RATE, buffer);
        assert_eq!(buffer[1], 0.0);
        assert_eq!(buffer[2], 0.0);
    }
}
