#[macro_use]
extern crate galvanic_test;
extern crate jack;

mod areas;
mod evdev;
mod generator;
mod run_jack;

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

impl<E: std::error::Error> From<E> for AppError {
    fn from(e: E) -> Self {
        AppError::AppError {
            description: String::from(e.description()),
        }
    }
}

fn main() -> Result<(), AppError> {
    let mutex = Arc::new(Mutex::new(Generator::new(300.0, |phase| {
        if phase < PI {
            -1.0
        } else {
            1.0
        }
    })));
    let _active_client = run_jack_generator(mutex.clone()).map_err(AppError::JackError)?;
    let file = "/dev/input/event15";
    let touches = Positions::new(file)?;
    let areas = Areas::new(1200, 36);
    areas.spawn_ui();
    let frequencies = Frequencies::new(areas, touches);
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
