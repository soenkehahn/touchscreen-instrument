#[macro_use]
extern crate galvanic_test;
extern crate jack;

mod generator;
mod input;
mod run_jack;

use generator::Generator;
use input::MouseInput;
use run_jack::run_jack_generator;
use std::clone::Clone;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::*;

#[derive(Debug)]
enum AppError {
    JackError(jack::Error),
    IOError(std::io::Error),
}

impl From<jack::Error> for AppError {
    fn from(e: jack::Error) -> Self {
        AppError::JackError(e)
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::IOError(e)
    }
}

fn main() -> Result<(), AppError> {
    let mutex = Arc::new(Mutex::new(Generator::new(300.0)));
    let _active_client = run_jack_generator(mutex.clone())?;
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
