use std::io::Error;

fn main_() -> Result<(), Error> {
    Ok(())
}

fn main() {
    match main_() {
        Ok(()) => {}
        Err(e) => {
            panic!(e);
        }
    }
}
