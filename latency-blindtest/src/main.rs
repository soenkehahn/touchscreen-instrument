extern crate rand;

use std::process::Command;

fn main() -> Result<(), std::io::Error> {
    set_period(get_random_period())?;
    Ok(())
}

fn set_period(period: i32) -> Result<(), std::io::Error> {
    let output = Command::new("jack_bufsize")
        .arg(format!("{}", period))
        .output()?;
    println!("{}", String::from_utf8_lossy(&output.stdout));
    println!("{}", String::from_utf8_lossy(&output.stderr));
    Ok(())
}

fn get_random_period() -> i32 {
    match rand::random::<u8>() % 2 {
        0 => 512,
        1 => 256,
        n => panic!(format!("not covered: {}", n)),
    }
}
