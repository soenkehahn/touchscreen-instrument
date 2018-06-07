extern crate jack;

use super::generator;
use super::generator::Generator;
use get_binary_name;
use jack::*;
use std::sync::{Arc, Mutex};
use std::*;
use ErrorString;

pub struct ActiveClient {
    _client: AsyncClient<(), ProcessHandler_>,
    pub generator_mutex: Arc<Mutex<Generator>>,
}

pub fn run_generator<F>(generator_args: generator::Args<F>) -> Result<ActiveClient, ErrorString>
where
    F: Fn(f32) -> f32 + 'static + Send,
{
    let name = get_binary_name()?;
    let (client, _status) = jack::Client::new(&name, jack::ClientOptions::NO_START_SERVER)?;
    let generator = Generator::new(generator_args, client.sample_rate() as i32);
    let mutex = Arc::new(Mutex::new(generator));

    let left_port = client.register_port("left-output", AudioOut)?;
    let right_port = client.register_port("right-output", AudioOut)?;

    let notification_handler = ();
    let process_handler = ProcessHandler_ {
        ports: Stereo {
            left: left_port,
            right: right_port,
        },
        generator: mutex.clone(),
    };
    let async_client = client.activate_async(notification_handler, process_handler)?;
    Ok(ActiveClient {
        _client: async_client,
        generator_mutex: mutex,
    })
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
