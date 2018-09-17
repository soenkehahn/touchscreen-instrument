extern crate clap;

use self::clap::{App, Arg};
use std::ffi::OsString;
use std::fmt::Display;
use std::str::FromStr;
use ErrorString;
use LayoutType;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Args {
    pub volume: f32,
    pub layout_type: LayoutType,
    pub start_note: i32,
    pub midi: bool,
    pub dev_mode: bool,
}

pub fn parse<S, T>(binary_name: String, args: T) -> Result<Args, ErrorString>
where
    S: Into<OsString> + Clone,
    T: Iterator<Item = S>,
{
    let layout_help = format!(
        "layout type, possible values: {:?}, (default: {:?})",
        LayoutType::iter_variants().collect::<Vec<LayoutType>>(),
        LayoutType::default()
    );
    let app = App::new(binary_name)
        .version("0.1.0")
        .author("Sönke Hahn <soenkehahn@gmail.com>")
        .about("musical instrument for touch screens")
        .arg(
            Arg::with_name("volume")
                .long("volume")
                .value_name("VOLUME")
                .help("Sets a custom sound volume (default: 1.0)")
                .takes_value(true),
        ).arg(
            Arg::with_name("layout")
                .long("layout")
                .value_name("LAYOUT_TYPE")
                .help(&layout_help)
                .takes_value(true),
        ).arg(
            Arg::with_name("pitch")
                .long("pitch")
                .value_name("MIDI")
                .help("Sets a custom midi pitch to start from (default: 36)")
                .takes_value(true),
        ).arg(
            Arg::with_name("midi")
                .long("midi")
                .help("switches to the midi backend (default: false)")
                .takes_value(false),
        ).arg(
            Arg::with_name("dev-mode")
                .long("dev-mode")
                .help("disables touch input and audio output (default: false)")
                .takes_value(false),
        );
    let matches = app.get_matches_from(args);
    Ok(Args {
        volume: parse_with_default(matches.value_of("volume"), 1.0)?,
        layout_type: parse_layout_type(matches.value_of("layout"))?,
        start_note: parse_with_default(matches.value_of("pitch"), 36)?,
        midi: matches.is_present("midi"),
        dev_mode: matches.is_present("dev-mode"),
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

fn parse_layout_type(input: Option<&str>) -> Result<LayoutType, ErrorString> {
    match input {
        None => Ok(LayoutType::default()),
        Some("Stripes") => Ok(LayoutType::Stripes),
        Some("Peas") => Ok(LayoutType::Peas),
        Some("Triangles") => Ok(LayoutType::Triangles),
        Some("Parallelograms") => Ok(LayoutType::Parallelograms),
        Some(layout) => Err(ErrorString(format!(
            "unknown layout: {}, possible values: {:?}",
            layout,
            LayoutType::iter_variants().collect::<Vec<LayoutType>>()
        ))),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn args(extra_args: Vec<&str>) -> Args {
        let with_binary = {
            let mut result = vec!["test-binary-name"];
            result.append(&mut extra_args.clone());
            result
        };
        parse("test-binary-name".to_string(), with_binary.into_iter()).unwrap()
    }

    #[test]
    fn has_default_values_when_no_arguments_given() {
        let expected = Args {
            volume: 1.0,
            layout_type: LayoutType::default(),
            start_note: 36,
            midi: false,
            dev_mode: false,
        };
        assert_eq!(args(vec![]), expected)
    }

    #[test]
    fn allows_to_change_the_volume() {
        assert_eq!(args(vec!["--volume", "2"]).volume, 2.0);
    }

    #[test]
    fn allows_to_change_the_layout_type() {
        assert_eq!(
            args(vec!["--layout", "Triangles"]).layout_type,
            LayoutType::Triangles
        );
    }

    #[test]
    fn allows_to_change_the_pitch() {
        assert_eq!(args(vec!["--pitch", "48"]).start_note, 48);
    }

    #[test]
    fn allows_to_change_to_the_midi_backend() {
        assert_eq!(args(vec!["--midi"]).midi, true);
    }

    #[test]
    fn allows_to_enable_dev_mode() {
        assert_eq!(args(vec!["--dev-mode"]).dev_mode, true);
    }
}
