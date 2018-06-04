extern crate clap;
extern crate jack;

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
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub enum AppError {
    JackError(jack::Error),
    AppError { description: String },
}

impl AppError {
    fn new(description: String) -> AppError {
        AppError::AppError { description }
    }
}

fn to_app_error<T>(option: Option<T>, message: &str) -> Result<T, AppError> {
    option.ok_or(AppError::new(message.to_string()))
}

impl<E: std::error::Error> From<E> for AppError {
    fn from(e: E) -> Self {
        AppError::AppError {
            description: String::from(e.description()),
        }
    }
}

fn get_binary_name() -> Result<String, AppError> {
    let current_exe = std::env::current_exe()?;
    let binary_name = to_app_error(
        to_app_error(current_exe.file_name(), "invalid current executable")?.to_str(),
        "executable not valid unicode",
    )?;
    Ok(binary_name.to_string())
}

fn main() -> Result<(), AppError> {
    let cli_args = cli::parse(clap::App::new(get_binary_name()?)).map_err(AppError::new)?;
    let mutex = Arc::new(Mutex::new(Generator::new(300.0, move |phase| {
        cli_args.volume * if phase < PI { -1.0 } else { 1.0 }
    })));
    let _active_client =
        run_jack_generator(get_binary_name()?, mutex.clone()).map_err(AppError::JackError)?;
    let file = "/dev/input/event15";
    let touches = Positions::new(file)?;
    let areas = Areas::new(
        800,
        cli_args.start_note,
        SCREEN_WIDTH as f32 / 16383.0,
        SCREEN_HEIGHT as f32 / 9570.0,
    );
    areas.spawn_ui();
    let frequencies = Frequencies::new(areas, touches.map(|touchstates| touchstates[0].clone()));
    for frequency_update in frequencies {
        match mutex.lock() {
            Err(e) => {
                println!("main_: error: {:?}", e);
            }
            Ok(mut generator) => match frequency_update {
                TouchState::NoTouch => {
                    generator.muted = true;
                }
                TouchState::Touch(frequency) => {
                    generator.muted = false;
                    generator.frequency = frequency;
                }
            },
        }
    }
    Ok(())
}
