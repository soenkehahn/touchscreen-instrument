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

    let port = client.register_port("mono-output", AudioOut)?;

    let notification_handler = ();
    let process_handler = Generator { port: port };
    let _active_client = client.activate_async(notification_handler, process_handler)?;
    sleep_forever();
    Ok(())
}

fn sleep_forever() {
    loop {
        thread::sleep(time::Duration::new(100, 0));
    }
}

struct Generator {
    port: Port<AudioOut>,
}

impl ProcessHandler for Generator {
    fn process(&mut self, _client: &Client, scope: &ProcessScope) -> Control {
        let mut rng = rand::thread_rng();
        let buffer: &mut [f32] = self.port.as_mut_slice(scope);
        for sample_index in 0..buffer.len() {
            buffer[sample_index] = rng.gen_range(-1.0, 1.0) * 0.1;
        }
        Control::Continue
    }
}
