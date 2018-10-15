extern crate jack;
extern crate skipchannel;

use self::skipchannel::*;
use super::generator;
use super::generator::Generator;
use super::logger::Logger;
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
    _async_client: AsyncClient<Logger, AudioProcessHandler>,
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

        let logger = Logger::new_and_spawn();
        let (sender, receiver) = skipchannel();
        let process_handler = AudioProcessHandler {
            logger: logger.clone(),
            ports,
            receiver,
            generators,
        };
        let async_client = client.activate_async(logger, process_handler)?;
        let audio_player = AudioPlayer {
            _async_client: async_client,
            sender,
        };
        audio_player.set_period(512)?;
        Ok(audio_player)
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
    logger: Logger,
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

    fn fill_buffer(
        logger: &Logger,
        client: &Client,
        generators: &mut Slots<Generator>,
        buffer: &mut [f32],
    ) {
        for sample in buffer.iter_mut() {
            *sample = 0.0;
        }
        for generator in generators.iter_mut() {
            generator.generate(client.sample_rate() as i32, buffer);
        }
        logger.check_clipping(buffer);
    }

    fn fill_buffers(&mut self, client: &Client, scope: &ProcessScope) {
        let left_buffer: &mut [f32] = self.ports.left.as_mut_slice(scope);
        AudioProcessHandler::fill_buffer(&self.logger, client, &mut self.generators, left_buffer);
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
