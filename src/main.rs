extern crate clap;
extern crate jack;
extern crate nix;

mod areas;
mod cli;
mod evdev;
mod generator;
mod run_jack;

use areas::{Areas, Frequencies, NoteEvent};
use evdev::*;
use run_jack::run_generator;
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

fn main() -> Result<(), ErrorString> {
    let cli_args = cli::parse(clap::App::new(get_binary_name()?))?;
    let generator_args = generator::Args {
        amplitude: cli_args.volume,
        decay: 0.005,
        wave_form: move |phase| if phase < PI { -1.0 } else { 1.0 },
    };
    let active_client = run_generator(generator_args)?;
    let touches = Positions::new("/dev/input/event15")?;
    let areas = Areas::new(TOUCH_WIDTH, TOUCH_HEIGHT, 800, cli_args.start_note);
    areas.clone().spawn_ui();
    let frequencies = Frequencies::new(
        areas,
        touches.map(|touchstates| *TouchState::get_first(touchstates.iter())),
    );
    for frequency_update in frequencies {
        match active_client.generator_mutex.lock() {
            Err(e) => {
                eprintln!("main_: error: {:?}", e);
            }
            Ok(mut generator) => match frequency_update {
                NoteEvent::NoteOff => {
                    generator.note_off();
                }
                NoteEvent::NoteOn(frequency) => {
                    generator.note_on(frequency);
                }
            },
        }
    }
    Ok(())
}
