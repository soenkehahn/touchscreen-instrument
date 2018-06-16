extern crate clap;

use self::clap::{App, Arg};
use std::fmt::Display;
use std::str::FromStr;
use ErrorString;

#[derive(Debug)]
pub struct Args {
    pub volume: f32,
    pub start_note: i32,
    pub midi: bool,
}

pub fn parse<'a, 'b>(app: App<'a, 'b>) -> Result<Args, ErrorString> {
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
        .arg(
            Arg::with_name("midi")
                .long("midi")
                .help("switches to the midi backend (default: false)")
                .takes_value(false),
        )
        .get_matches();
    let volume: f32 = parse_with_default(matches.value_of("volume"), 1.0)?;
    let start_note: i32 = parse_with_default(matches.value_of("pitch"), 36)?;
    let midi = matches.is_present("midi");
    Ok(Args {
        volume,
        start_note,
        midi,
    })
}

fn parse_with_default<N>(input: Option<&str>, default: N) -> Result<N, ErrorString>
where
    N: FromStr,
    <N as FromStr>::Err: Display,
{
    match input {
        None => Ok(default),
        Some(string) => string
            .parse()
            .map_err(|e| ErrorString::from(format!("{}", e))),
    }
}
