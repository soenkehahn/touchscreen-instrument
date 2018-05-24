#![feature(type_ascription)]

extern crate jack;
extern crate rand;

use jack::*;
use rand::Rng;
use std::*;

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
    let process_handler = Generator {
        ports: Stereo {
            left: left_port,
            right: right_port,
        },
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

struct Generator {
    ports: Stereo<Port<AudioOut>>,
}

impl ProcessHandler for Generator {
    fn process(&mut self, _client: &Client, scope: &ProcessScope) -> Control {
        let mut rng = rand::thread_rng();
        let left_buffer: &mut [f32] = self.ports.left.as_mut_slice(scope);
        let right_buffer: &mut [f32] = self.ports.right.as_mut_slice(scope);
        for sample_index in 0..left_buffer.len() {
            let sample = rng.gen_range(-1.0, 1.0) * 0.1;
            left_buffer[sample_index] = sample;
            right_buffer[sample_index] = sample;
        }
        Control::Continue
    }
}
