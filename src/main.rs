#[macro_use]
extern crate galvanic_test;
extern crate jack;

mod evdev;
mod generator;
mod input;
mod run_jack;

use evdev::Events;
use generator::Generator;
use input::MouseInput;
use run_jack::run_jack_generator;
use std::clone::Clone;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::*;

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
    fork_evdev_logging();
    let mutex = Arc::new(Mutex::new(Generator::new(300.0)));
    let _active_client = run_jack_generator(mutex.clone()).map_err(AppError::JackError)?;
    let mouse_input = MouseInput::new(File::open("/dev/input/mice")?);
    mouse_input.for_each(|position| {
        let frequency = 300.0 + position.x as f32;
        match mutex.lock() {
            Err(e) => {
                println!("main_: error: {:?}", e);
            }
            Ok(mut generator) => {
                generator.frequency = frequency;
            }
        }
    });
    Ok(())
}

fn fork_evdev_logging() {
    thread::spawn(|| {
        for event in Events::new("/dev/input/event15").unwrap() {
            println!("{:?}", event);
        }
    });
}
