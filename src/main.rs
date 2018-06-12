#![cfg_attr(feature = "ci", deny(warnings))]

extern crate clap;
extern crate jack;
extern crate nix;

mod areas;
mod cli;
mod evdev;
mod sound;

use areas::{note_event_source::NoteEventSource, Areas};
use evdev::*;
use sound::audio_player::AudioPlayer;
use sound::generator;
use sound::midi_player::MidiPlayer;
use sound::Player;
use std::clone::Clone;
use std::f32::consts::PI;
use std::fmt::Debug;

const TOUCH_WIDTH: u32 = 16383;
const TOUCH_HEIGHT: u32 = 9570;

pub struct ErrorString(String);

impl Debug for ErrorString {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let ErrorString(string) = self;
        string.fmt(f)
    }
}

trait AddMessage<T>: Sized {
    fn add_message(self, message: String) -> Result<T, ErrorString>;
}

impl<T, E> AddMessage<T> for Result<T, E>
where
    ErrorString: From<E>,
{
    fn add_message(self, message: String) -> Result<T, ErrorString> {
        self.map_err(|e| {
            let ErrorString(string) = ErrorString::from(e);
            ErrorString(format!("{}: {}", message, string))
        })
    }
}

impl From<std::io::Error> for ErrorString {
    fn from(e: std::io::Error) -> ErrorString {
        ErrorString(format!("{}", e))
    }
}

impl From<String> for ErrorString {
    fn from(e: String) -> ErrorString {
        ErrorString(format!("{}", e))
    }
}

impl From<&'static str> for ErrorString {
    fn from(e: &str) -> ErrorString {
        ErrorString(format!("{}", e))
    }
}

impl From<nix::Errno> for ErrorString {
    fn from(e: nix::Errno) -> ErrorString {
        ErrorString(format!("{}", e))
    }
}

impl From<jack::Error> for ErrorString {
    fn from(e: jack::Error) -> ErrorString {
        ErrorString(format!("{:?}", e))
    }
}

fn get_binary_name() -> Result<String, ErrorString> {
    let current_exe = std::env::current_exe()?;
    let binary_name = current_exe
        .file_name()
        .ok_or("invalid current executable")?
        .to_str()
        .ok_or("executable not valid unicode")?;
    Ok(binary_name.to_string())
}

fn get_note_event_source() -> Result<NoteEventSource, ErrorString> {
    let touches = PositionSource::new("/dev/input/event15")?;
    let areas = Areas::peas(TOUCH_WIDTH, TOUCH_HEIGHT, 1000);
    areas.clone().spawn_ui();
    Ok(NoteEventSource::new(areas, touches))
}

fn get_player(cli_args: cli::Args) -> Result<Box<Player>, ErrorString> {
    match cli_args.midi {
        false => {
            let generator_args = generator::Args {
                amplitude: cli_args.volume,
                decay: 0.005,
                wave_form: move |phase| if phase < PI { -1.0 } else { 1.0 },
            };
            Ok(Box::new(AudioPlayer::new(generator_args)?))
        }
        true => Ok(Box::new(MidiPlayer::new()?)),
    }
}

fn main() -> Result<(), ErrorString> {
    let cli_args = cli::parse(clap::App::new(get_binary_name()?))?;
    let note_event_source = get_note_event_source()?;
    let player = get_player(cli_args)?;
    player.consume(note_event_source);
    Ok(())
}
