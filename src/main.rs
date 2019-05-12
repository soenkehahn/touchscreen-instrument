#![cfg_attr(feature = "ci", deny(warnings))]

#[macro_use]
extern crate custom_derive;
#[macro_use]
extern crate enum_derive;

mod areas;
mod cli;
mod evdev;
mod sound;
mod utils;

use areas::layouts::{grid, grid2, parallelograms};
use areas::{note_event_source::NoteEventSource, Areas};
use evdev::*;
use sound::audio_player::AudioPlayer;
use sound::generator;
use sound::hammond::mk_hammond;
use sound::midi_player::MidiPlayer;
use sound::wave_form::WaveForm;
use sound::Player;
use std::clone::Clone;
use std::f32::consts::PI;
use std::fmt::Debug;
use std::process::exit;

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
        ErrorString(e)
    }
}

impl From<&'static str> for ErrorString {
    fn from(e: &str) -> ErrorString {
        ErrorString(e.to_string())
    }
}

impl From<nix::errno::Errno> for ErrorString {
    fn from(e: nix::errno::Errno) -> ErrorString {
        ErrorString(format!("{}", e))
    }
}

impl From<jack::Error> for ErrorString {
    fn from(e: jack::Error) -> ErrorString {
        ErrorString(format!("{:?}", e))
    }
}

impl From<std::num::ParseFloatError> for ErrorString {
    fn from(e: std::num::ParseFloatError) -> ErrorString {
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
#[derive(Debug, Clone, Copy, IterVariants(LayoutTypeVariants), PartialEq)]
    pub enum LayoutType {
        Parallelograms,
        Grid,
        Grid2,
    }
}

impl Default for LayoutType {
    fn default() -> LayoutType {
        LayoutType::Grid2
    }
}

fn get_areas(layout_type: LayoutType) -> Areas {
    match layout_type {
        LayoutType::Parallelograms => parallelograms(TOUCH_WIDTH as i32, TOUCH_HEIGHT as i32),
        LayoutType::Grid => grid(TOUCH_WIDTH as i32, TOUCH_HEIGHT as i32, 16, 11, 36),
        LayoutType::Grid2 => grid2(TOUCH_WIDTH as i32, TOUCH_HEIGHT as i32),
    }
}

fn get_note_event_source(cli_args: &cli::Args) -> Result<NoteEventSource, ErrorString> {
    let areas = get_areas(cli_args.layout_type);
    areas.clone().spawn_ui(cli_args);
    let touches = if cli_args.dev_mode {
        PositionSource::blocking()
    } else {
        PositionSource::new("/dev/input/by-id/usb-ILITEK_Multi-Touch-V5100-event-if00")?
    };
    Ok(NoteEventSource::new(areas, touches))
}

fn get_player(cli_args: &cli::Args) -> Result<Box<dyn Player>, ErrorString> {
    if cli_args.midi {
        Ok(Box::new(MidiPlayer::new()?))
    } else {
        let generator_args = generator::Args {
            amplitude: cli_args.volume,
            attack: 0.005,
            release: 0.005,
            wave_form: mk_wave_form(cli_args),
        };
        Ok(Box::new(AudioPlayer::new(generator_args)?))
    }
}

fn mk_wave_form(cli_args: &cli::Args) -> WaveForm {
    match cli_args.sound {
        cli::Sound::Rectangle => WaveForm::new(|phase| if phase < PI { -1.0 } else { 1.0 }),
        cli::Sound::Harmonics(ref harmonics) => mk_hammond(harmonics.clone()),
    }
}

fn run() -> Result<(), ErrorString> {
    let cli_args = &cli::parse(get_binary_name()?, std::env::args())?;
    let note_event_source = get_note_event_source(cli_args)?;
    let player = get_player(cli_args)?;
    player.consume(note_event_source);
    Ok(())
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(ErrorString(message)) => {
            eprintln!("{}", message);
            exit(1);
        }
    }
}
