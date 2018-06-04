extern crate jack;

use super::generator::Generator;
use jack::*;
use std::sync::{Arc, Mutex};
use std::*;

pub fn run_jack_generator(
    name: String,
    generator: Arc<Mutex<Generator>>,
) -> Result<AsyncClient<(), ProcessHandler_>, Error> {
    let (client, _status) =
        jack::Client::new(&name.to_string(), jack::ClientOptions::NO_START_SERVER)?;

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
    client.activate_async(notification_handler, process_handler)
}

struct Stereo<Port> {
    left: Port,
    right: Port,
}

pub struct ProcessHandler_ {
    ports: Stereo<Port<AudioOut>>,
    generator: Arc<Mutex<Generator>>,
}

impl ProcessHandler for ProcessHandler_ {
    fn process(&mut self, client: &Client, scope: &ProcessScope) -> Control {
        match self.generator.lock() {
            Ok(mut generator) => {
                let left_buffer: &mut [f32] = self.ports.left.as_mut_slice(scope);
                generator.generate(client.sample_rate() as i32, left_buffer);
                self.ports
                    .right
                    .as_mut_slice(scope)
                    .copy_from_slice(left_buffer);
            }
            Err(e) => {
                eprintln!("process: error: {:?}", e);
            }
        }
        Control::Continue
    }
}
