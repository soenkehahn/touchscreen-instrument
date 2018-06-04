extern crate clap;
extern crate jack;
extern crate nix;

mod areas;
mod cli;
mod evdev;
mod generator;
mod run_jack;

use areas::render::{SCREEN_HEIGHT, SCREEN_WIDTH};
use areas::{Areas, Frequencies};
use evdev::*;
use generator::Generator;
use run_jack::run_jack_generator;
use std::clone::Clone;
use std::f32::consts::PI;
use std::fmt::Debug;
use std::sync::{Arc, Mutex};

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
    let mutex = Arc::new(Mutex::new(Generator::new(move |phase| {
        cli_args.volume * if phase < PI { -1.0 } else { 1.0 }
    })));
    let _active_client = run_jack_generator(get_binary_name()?, mutex.clone())?;
    let file = "/dev/input/event15";
    let touches = Positions::new(file)?;
    let areas = Areas::new(
        800,
        cli_args.start_note,
        SCREEN_WIDTH as f32 / 16383.0,
        SCREEN_HEIGHT as f32 / 9570.0,
    );
    areas.spawn_ui();
    let frequencies = Frequencies::new(
        areas,
        touches.map(|touchstates| *TouchState::get_first(touchstates.iter())),
    );
    for frequency_update in frequencies {
        match mutex.lock() {
            Err(e) => {
                eprintln!("main_: error: {:?}", e);
            }
            Ok(mut generator) => match frequency_update {
                TouchState::NoTouch => {
                    generator.frequency = None;
                }
                TouchState::Touch(frequency) => {
                    generator.frequency = Some(frequency);
                }
            },
        }
    }
    Ok(())
}
