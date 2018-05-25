#[macro_use]
extern crate galvanic_test;
extern crate jack;

mod generator;
mod input;
mod run_jack;

use generator::Generator;
use jack::*;
use run_jack::run_jack_generator;

fn main() {
    match main_() {
        Ok(()) => {}
        Err(e) => {
            panic!("error thrown: {:?}", e);
        }
    }
}

fn main_() -> Result<(), Error> {
    run_jack_generator(Generator::new(300.0))
}
