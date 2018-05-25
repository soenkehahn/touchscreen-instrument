extern crate jack;

use super::generator::Generator;
use jack::*;
use std::*;

pub fn run_jack_generator(generator: Generator) -> Result<(), Error> {
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
        generator,
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
