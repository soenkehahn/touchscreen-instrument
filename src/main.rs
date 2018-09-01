#![cfg_attr(feature = "ci", deny(warnings))]

extern crate clap;
extern crate jack;
extern crate nix;

#[macro_use]
extern crate custom_derive;
#[macro_use]
extern crate enum_derive;

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

const TOUCH_WIDTH: i32 = 16383;
const TOUCH_HEIGHT: i32 = 9570;

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

custom_derive! {
#[derive(Debug, Clone, Copy, IterVariants(LayoutTypeVariants))]
    pub enum LayoutType {
        Stripes,
        Peas,
        Triangles,
        Parallelograms,
    }
}

impl Default for LayoutType {
    fn default() -> LayoutType {
        LayoutType::Parallelograms
    }
}

fn get_areas(cli_args: cli::Args) -> Areas {
    match cli_args.layout_type {
        LayoutType::Stripes => Areas::stripes(TOUCH_WIDTH, TOUCH_HEIGHT, 1000, cli_args.start_note),
        LayoutType::Peas => Areas::peas(TOUCH_WIDTH, TOUCH_HEIGHT, 1400),
        LayoutType::Triangles => Areas::triangles(TOUCH_WIDTH as i32, TOUCH_HEIGHT as i32, 1400),
        LayoutType::Parallelograms => Areas::parallelograms(
            TOUCH_WIDTH as i32,
            TOUCH_HEIGHT as i32,
            (1000, 1300),
            200,
            24,
            5,
        ),
    }
}

fn get_note_event_source(cli_args: cli::Args) -> Result<NoteEventSource, ErrorString> {
    let touches = PositionSource::new("/dev/input/by-id/usb-ILITEK_Multi-Touch-V5100-event-if00")?;
    let areas = get_areas(cli_args);
    areas.clone().spawn_ui(cli_args);
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
    if cli_args.dev_mode {
        get_areas(cli_args).run_ui(cli_args);
    } else {
        let note_event_source = get_note_event_source(cli_args)?;
        let player = get_player(cli_args)?;
        player.consume(note_event_source);
    }
    Ok(())
}
