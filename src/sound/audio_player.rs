extern crate jack;

use super::generator;
use super::generator::Generator;
use super::xrun_logger::XRunLogger;
use super::Player;
use areas::note_event_source::NoteEventSource;
use evdev::{slot_map, Slots};
use get_binary_name;
use jack::*;
use sound::NoteEvent;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::*;
use ErrorString;

pub struct AudioPlayer {
    async_client: AsyncClient<XRunLogger, AudioProcessHandler>,
    pub generators_mutex: Arc<Mutex<Slots<Generator>>>,
}

impl AudioPlayer {
    pub fn new<F>(generator_args: generator::Args<F>) -> Result<AudioPlayer, ErrorString>
    where
        F: Fn(f32) -> f32 + 'static + Send + Clone,
    {
        let name = get_binary_name()?;
        let (client, _status) = jack::Client::new(&name, jack::ClientOptions::empty())?;
        let generators = slot_map(generator_args.unfold_generator_args(), |args| {
            Generator::new((*args).clone(), client.sample_rate() as i32)
        });
        let generators_mutex = Arc::new(Mutex::new(generators));
        let ports = Stereo {
            left: client.register_port("left-output", AudioOut)?,
            right: client.register_port("right-output", AudioOut)?,
        };
        let port_clones = Stereo {
            left: ports.left.clone_unowned(),
            right: ports.right.clone_unowned(),
        };

        let notification_handler = XRunLogger::new_and_spawn();
        let process_handler = AudioProcessHandler {
            ports,
            generators_mutex: generators_mutex.clone(),
        };
        let async_client = client.activate_async(notification_handler, process_handler)?;
        let audio_player = AudioPlayer {
            async_client,
            generators_mutex: generators_mutex.clone(),
        };
        audio_player.connect_to_system_ports(port_clones)?;
        audio_player.set_period(512)?;
        Ok(audio_player)
    }

    fn connect_to_system_ports(&self, ports: Stereo<Port<Unowned>>) -> Result<(), ErrorString> {
        self.connect_to_port(&ports.left, "system:playback_1")?;
        self.connect_to_port(&ports.right, "system:playback_2")?;
        Ok(())
    }

    fn connect_to_port(&self, source_port: &Port<Unowned>, name: &str) -> Result<(), ErrorString> {
        let destination_port = self
            .async_client
            .as_client()
            .port_by_name(name)
            .ok_or(format!("Couldn't find audio port {}", name))?;
        self.async_client
            .as_client()
            .connect_ports(source_port, &destination_port)?;
        Ok(())
    }

    fn set_period(&self, period: i32) -> Result<(), ErrorString> {
        let output = Command::new("jack_bufsize")
            .arg(format!("{}", period))
            .output()?;
        println!("{}", String::from_utf8_lossy(&output.stdout));
        println!("{}", String::from_utf8_lossy(&output.stderr));
        Ok(())
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
