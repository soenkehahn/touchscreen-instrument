extern crate jack;
#[macro_use]
extern crate galvanic_test;

use jack::*;
use std::*;

const TAU: f32 = 6.2831855;

fn main() {
    match main_() {
        Ok(()) => {}
        Err(e) => {
            panic!("error thrown: {:?}", e);
        }
    }
}

fn main_() -> Result<(), Error> {
    let (client, _status) =
        jack::Client::new("my-rust-client", jack::ClientOptions::NO_START_SERVER)?;

    let left_port = client.register_port("left-output", AudioOut)?;
    let right_port = client.register_port("right-output", AudioOut)?;

    let notification_handler = ();
    let process_handler = ProcessHandler_ {
        ports: Stereo {
            left: left_port,
            right: right_port,
        },
        generator: Generator::new(300.0),
    };
    let _active_client = client.activate_async(notification_handler, process_handler)?;
    sleep_forever();
    Ok(())
}

fn sleep_forever() {
    loop {
        thread::sleep(time::Duration::new(100, 0));
    }
}

struct Stereo<Port> {
    left: Port,
    right: Port,
}

struct ProcessHandler_ {
    ports: Stereo<Port<AudioOut>>,
    generator: Generator,
}

struct Generator {
    frequency: f32,
    phase: f32,
}

impl Generator {
    fn new(frequency: f32) -> Generator {
        Generator {
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

    fn generate(&mut self, sample_rate: i32, buffer: &mut [f32]) {
        for sample_index in 0..buffer.len() {
            let sample = self.phase.sin();
            buffer[sample_index] = sample;
            self.crank_phase(sample_rate);
        }
    }
}

impl ProcessHandler for ProcessHandler_ {
    fn process(&mut self, _client: &Client, scope: &ProcessScope) -> Control {
        let left_buffer: &mut [f32] = self.ports.left.as_mut_slice(scope);
        let right_buffer: &mut [f32] = self.ports.right.as_mut_slice(scope);
        self.generator
            .generate(_client.sample_rate() as i32, left_buffer);
        for sample_index in 0..right_buffer.len() {
            right_buffer[sample_index] = left_buffer[sample_index];
        }
        Control::Continue
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
            Generator::new(1.0)
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

    test it_allows_to_change_the_frequency(generator) {
        let buffer: &mut [f32] = &mut [42.0; 10];
        generator.val.frequency = 300.0;
        generator.val.generate(SAMPLE_RATE, buffer);
        assert_eq!(buffer[1], (300.0 * TAU / SAMPLE_RATE as f32).sin());
        assert_eq!(buffer[2], (2.0 * 300.0 * TAU / SAMPLE_RATE as f32).sin());
    }
}
