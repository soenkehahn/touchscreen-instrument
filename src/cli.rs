extern crate clap;

use self::clap::{App, Arg};
use std::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Clone, Copy)]
pub struct CliArgs {
    pub volume: f32,
    pub start_note: i32,
}

pub fn parse<'a, 'b>(app: App<'a, 'b>) -> Result<CliArgs, String> {
    let matches = app.version("0.1.0")
        .author("Sönke Hahn <soenkehahn@gmail.com>")
        .about("musical instrument for touch screens")
        .arg(
            Arg::with_name("volume")
                .long("volume")
                .value_name("VOLUME")
                .help("Sets a custom sound volume (default: 1.0)")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("pitch")
                .long("pitch")
                .value_name("MIDI")
                .help("Sets a custom midi pitch to start from (default: 36)")
                .takes_value(true),
        )
        .get_matches();
    let volume: f32 = parse_with_default(matches.value_of("volume"), 1.0)?;
    let start_note: i32 = parse_with_default(matches.value_of("pitch"), 36)?;
    Ok(CliArgs { volume, start_note })
}

fn parse_with_default<N>(input: Option<&str>, default: N) -> Result<N, String>
where
    N: FromStr,
    <N as FromStr>::Err: Display,
{
    match input {
        None => Ok(default),
        Some(string) => string.parse().map_err(|e| format!("{}", e)),
    }
}
