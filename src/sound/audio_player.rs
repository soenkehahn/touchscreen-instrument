use super::generator::Generators;
use super::logger::Logger;
use super::Player;
use crate::cli;
use crate::get_binary_name;
use crate::sound::midi_controller::MidiController;
use crate::sound::{NoteEvent, NoteEventSource, POLYPHONY};
use crate::ErrorString;
use jack::*;
use skipchannel::*;
use std::*;

pub struct AudioPlayer {
    _async_client: AsyncClient<Logger, AudioProcessHandler>,
    sender: Sender<[NoteEvent; POLYPHONY]>,
}

impl AudioPlayer {
    pub fn new(cli_args: &cli::Args) -> Result<AudioPlayer, ErrorString> {
        let name = get_binary_name()?;
        let (client, _status) = jack::Client::new(&name, jack::ClientOptions::empty())?;
        let midi_controller = MidiController::new(&client)?;
        let generators = Generators::new(cli_args);
        let audio_ports = Stereo {
            left: client.register_port("left-output", AudioOut)?,
            right: client.register_port("right-output", AudioOut)?,
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
            _async_client: async_client,
            sender,
        };
        Ok(audio_player)
    }
}

impl Player for AudioPlayer {
    fn consume(&self, note_event_source: NoteEventSource) {
        for voices in note_event_source {
            self.sender.send(voices);
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
    receiver: Receiver<[NoteEvent; POLYPHONY]>,
    generators: Generators,
}

impl AudioProcessHandler {
    fn handle_events(&mut self, scope: &ProcessScope) {
        self.midi_controller
            .handle_events(&mut self.generators, scope);
        self.handle_note_events();
    }

    fn handle_note_events(&mut self) {
        if let Some(voices) = self.receiver.recv() {
            Generators::handle_note_events(&mut self.generators, voices);
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
        generators: &mut Generators,
        buffer: &mut [f32],
    ) {
        for sample in buffer.iter_mut() {
            *sample = 0.0;
        }
        generators.generate(client.sample_rate(), buffer);
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
