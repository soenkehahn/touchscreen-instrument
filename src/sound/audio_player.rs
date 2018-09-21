extern crate jack;
extern crate skipchannel;

use self::skipchannel::*;
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
use std::*;
use ErrorString;

pub struct AudioPlayer {
    async_client: AsyncClient<XRunLogger, AudioProcessHandler>,
    sender: Sender<Slots<NoteEvent>>,
}

impl AudioPlayer {
    pub fn new(generator_args: generator::Args) -> Result<AudioPlayer, ErrorString> {
        let name = get_binary_name()?;
        let (client, _status) = jack::Client::new(&name, jack::ClientOptions::empty())?;
        let generators = slot_map(generator_args.unfold_generator_args(), |args| {
            Generator::new((*args).clone(), client.sample_rate() as i32)
        });
        let ports = Stereo {
            left: client.register_port("left-output", AudioOut)?,
            right: client.register_port("right-output", AudioOut)?,
        };
        let port_clones = Stereo {
            left: ports.left.clone_unowned(),
            right: ports.right.clone_unowned(),
        };

        let notification_handler = XRunLogger::new_and_spawn();
        let (sender, receiver) = skipchannel();
        let process_handler = AudioProcessHandler {
            ports,
            receiver,
            generators,
        };
        let async_client = client.activate_async(notification_handler, process_handler)?;
        let audio_player = AudioPlayer {
            async_client,
            sender,
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
        let destination_port = self.async_client
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
            self.sender.send(slots);
        }
    }
}

struct Stereo<Port> {
    left: Port,
    right: Port,
}

pub struct AudioProcessHandler {
    ports: Stereo<Port<AudioOut>>,
    receiver: Receiver<Slots<NoteEvent>>,
    generators: Slots<Generator>,
}

impl AudioProcessHandler {
    fn handle_events(&mut self) {
        match self.receiver.recv() {
            None => {}
            Some(slots) => {
                for (event, generator) in slots.into_iter().zip(self.generators.iter_mut()) {
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

    fn fill_buffer(client: &Client, generators: &mut Slots<Generator>, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = 0.0;
        }
        for generator in generators.iter_mut() {
            generator.generate(client.sample_rate() as i32, buffer);
        }
    }

    fn fill_buffers(&mut self, client: &Client, scope: &ProcessScope) {
        let left_buffer: &mut [f32] = self.ports.left.as_mut_slice(scope);
        AudioProcessHandler::fill_buffer(client, &mut self.generators, left_buffer);
        self.ports
            .right
            .as_mut_slice(scope)
            .copy_from_slice(left_buffer);
    }
}

impl ProcessHandler for AudioProcessHandler {
    fn process(&mut self, client: &Client, scope: &ProcessScope) -> Control {
        self.handle_events();
        self.fill_buffers(client, scope);
        Control::Continue
    }
}
