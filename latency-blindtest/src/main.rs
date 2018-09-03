extern crate rand;

use std::process::Command;

fn main() -> Result<(), std::io::Error> {
    let period = get_random_period();
    set_period(period)?;
    wait_for_enter()?;
    println!("last period: {}", period);
    Ok(())
}

fn wait_for_enter() -> Result<(), std::io::Error> {
    println!("press enter");
    let mut tmp = String::new();
    std::io::stdin().read_line(&mut tmp)?;
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
    let numbers: Vec<i32> = std::env::args()
        .skip(1)
        .map(|x| x.parse().unwrap())
        .collect();
    random_element(numbers)
}

fn random_element<T: Copy>(vec: Vec<T>) -> T {
    if vec.len() <= 0 {
        panic!("random_element: vector can't be empty");
    }
    let index: usize = rand::random::<usize>() % vec.len();
    *vec.get(index).unwrap()
}
