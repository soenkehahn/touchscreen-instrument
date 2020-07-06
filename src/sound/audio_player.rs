use super::generator;
use super::generator::Generator;
use super::logger::Logger;
use super::Player;
use crate::areas::note_event_source::NoteEventSource;
use crate::cli;
use crate::get_binary_name;
use crate::sound::midi_controller::MidiController;
use crate::sound::NoteEvent;
use crate::utils::{slot_map, Slots};
use crate::ErrorString;
use jack::*;
use skipchannel::*;
use std::*;

pub struct AudioPlayer {
    async_client: AsyncClient<Logger, AudioProcessHandler>,
    sender: Sender<Slots<NoteEvent>>,
}

impl AudioPlayer {
    pub fn new(
        cli_args: &cli::Args,
        generator_args: generator::Args,
    ) -> Result<AudioPlayer, ErrorString> {
        let name = get_binary_name()?;
        let (client, _status) = jack::Client::new(&name, jack::ClientOptions::empty())?;
        let midi_controller = MidiController::new(&client)?;
        let generators = slot_map(generator_args.unfold_generator_args(), |args| {
            Generator::new((*args).clone(), client.sample_rate() as i32)
        });
        let audio_ports = Stereo {
            left: client.register_port("left-output", AudioOut)?,
            right: client.register_port("right-output", AudioOut)?,
        };
        let port_clones = Stereo {
            left: audio_ports.left.clone_unowned(),
            right: audio_ports.right.clone_unowned(),
        };

        let logger = Logger::new_and_spawn();
        let (sender, receiver) = skipchannel();
        let process_handler = AudioProcessHandler {
            logger: logger.clone(),
            audio_ports,
            midi_controller,
            receiver,
            generators,
        };
        let async_client = client.activate_async(logger, process_handler)?;
        let audio_player = AudioPlayer {
            async_client,
            sender,
        };
        if cli_args.auto_connect {
            audio_player.connect_to_system_ports(port_clones)?;
        }
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
            .ok_or_else(|| format!("Couldn't find audio port {}", name))?;
        self.async_client
            .as_client()
            .connect_ports(source_port, &destination_port)?;
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
    audio_ports: Stereo<Port<AudioOut>>,
    midi_controller: MidiController,
    receiver: Receiver<Slots<NoteEvent>>,
    generators: Slots<Generator>,
}

impl AudioProcessHandler {
    fn handle_events(&mut self, scope: &ProcessScope) {
        self.midi_controller
            .handle_events(&mut self.generators, scope);
        self.handle_touch_events();
    }

    fn handle_touch_events(&mut self) {
        match self.receiver.recv() {
            None => {}
            Some(slots) => {
                for (event, generator) in slots.iter().zip(self.generators.iter_mut()) {
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

    fn fill_buffers(&mut self, client: &Client, scope: &ProcessScope) {
        let left_buffer: &mut [f32] = self.audio_ports.left.as_mut_slice(scope);
        AudioProcessHandler::fill_buffer(&self.logger, client, &mut self.generators, left_buffer);
        self.audio_ports
            .right
            .as_mut_slice(scope)
            .copy_from_slice(left_buffer);
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
}

impl ProcessHandler for AudioProcessHandler {
    fn process(&mut self, client: &Client, scope: &ProcessScope) -> Control {
        self.handle_events(scope);
        self.fill_buffers(client, scope);
        Control::Continue
    }
}
