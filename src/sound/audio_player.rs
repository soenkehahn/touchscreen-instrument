extern crate jack;

use super::generator;
use super::generator::Generator;
use super::Player;
use areas::note_event_source::NoteEventSource;
use evdev::{slot_map, Slots};
use get_binary_name;
use jack::*;
use sound::NoteEvent;
use std::sync::{Arc, Mutex};
use std::*;
use ErrorString;

pub struct AudioPlayer {
    _client: AsyncClient<(), AudioProcessHandler>,
    pub generators_mutex: Arc<Mutex<Slots<Generator>>>,
}

impl AudioPlayer {
    pub fn new<F>(generator_args: generator::Args<F>) -> Result<AudioPlayer, ErrorString>
    where
        F: Fn(f32) -> f32 + 'static + Send + Clone,
    {
        let name = get_binary_name()?;
        let (client, _status) = jack::Client::new(&name, jack::ClientOptions::NO_START_SERVER)?;
        let generators = slot_map(generator_args.unfold_generator_args(), |args| {
            Generator::new((*args).clone(), client.sample_rate() as i32)
        });
        let generators_mutex = Arc::new(Mutex::new(generators));

        let left_port = client.register_port("left-output", AudioOut)?;
        let right_port = client.register_port("right-output", AudioOut)?;

        let notification_handler = ();
        let process_handler = AudioProcessHandler {
            ports: Stereo {
                left: left_port,
                right: right_port,
            },
            generators_mutex: generators_mutex.clone(),
        };
        let async_client = client.activate_async(notification_handler, process_handler)?;
        Ok(AudioPlayer {
            _client: async_client,
            generators_mutex: generators_mutex.clone(),
        })
    }
}

impl Player for AudioPlayer {
    fn consume(&self, note_event_source: NoteEventSource) {
        for slots in note_event_source {
            match self.generators_mutex.lock() {
                Err(e) => {
                    eprintln!("main_: error: {:?}", e);
                }
                Ok(mut generators) => {
                    for (event, generator) in slots.into_iter().zip(generators.iter_mut()) {
                        match event {
                            NoteEvent::NoteOff => {
                                generator.note_off();
                            }
                            NoteEvent::NoteOn(frequency) => {
                                generator.note_on(*frequency);
                            }
                        }
                    }
                }
            }
        }
    }
}

struct Stereo<Port> {
    left: Port,
    right: Port,
}

pub struct AudioProcessHandler {
    ports: Stereo<Port<AudioOut>>,
    generators_mutex: Arc<Mutex<Slots<Generator>>>,
}

impl ProcessHandler for AudioProcessHandler {
    fn process(&mut self, client: &Client, scope: &ProcessScope) -> Control {
        match self.generators_mutex.lock() {
            Ok(mut generators) => {
                let left_buffer: &mut [f32] = self.ports.left.as_mut_slice(scope);
                for sample in left_buffer.iter_mut() {
                    *sample = 0.0;
                }
                for generator in generators.iter_mut() {
                    generator.generate(client.sample_rate() as i32, left_buffer);
                }
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
