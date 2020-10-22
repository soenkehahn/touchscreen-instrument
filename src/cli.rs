use crate::sound::midi_controller::{HarmonicVolume, HarmonicsState};
use crate::ErrorString;
use crate::LayoutType;
use clap::{App, Arg};
use std::ffi::OsString;

#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    pub volume: f32,
    pub layout_type: LayoutType,
    pub midi: bool,
    pub harmonics_state: HarmonicsState,
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
            Arg::with_name("harmonics")
                .long("harmonics")
                .help("sets the harmonics weights, separated by commas, e.g. '1,0.5,0.25' (default: 1)")
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
        volume: parse_volume(matches.value_of("volume"))?,
        layout_type: parse_layout_type(matches.value_of("layout"))?,
        harmonics_state: parse_harmonics_state(matches.value_of("harmonics"))?,
        midi: matches.is_present("midi"),
        dev_mode: matches.is_present("dev-mode"),
    })
}

fn parse_volume(input: Option<&str>) -> Result<f32, ErrorString> {
    match input {
        None => Ok(1.0),
        Some(string) => string
            .parse()
            .map_err(|e| ErrorString::from(format!("{}", e))),
    }
}

fn parse_layout_type(input: Option<&str>) -> Result<LayoutType, ErrorString> {
    match input {
        None => Ok(LayoutType::default()),
        Some("Parallelograms") => Ok(LayoutType::Parallelograms),
        Some("Grid") => Ok(LayoutType::Grid),
        Some("Grid2") => Ok(LayoutType::Grid2),
        Some(layout) => Err(ErrorString(format!(
            "unknown layout: {}, possible values: {:?}",
            layout,
            LayoutType::iter_variants().collect::<Vec<LayoutType>>()
        ))),
    }
}

fn parse_harmonics_state(input: Option<&str>) -> Result<HarmonicsState, ErrorString> {
    let mut result = HarmonicsState::new();
    match input {
        None => {
            result.set_harmonic_volume(HarmonicVolume {
                index: 0,
                volume: 1.0,
            });
        }
        Some(harmonics) => {
            for (index, harmonic) in harmonics.split(',').enumerate() {
                result.set_harmonic_volume(HarmonicVolume {
                    index,
                    volume: harmonic.parse()?,
                });
            }
        }
    };
    Ok(result)
}

#[cfg(test)]
pub mod test {
    use super::*;

    pub fn args(extra_args: Vec<&str>) -> Args {
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
            midi: false,
            harmonics_state: HarmonicsState {
                harmonics: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            },
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
        assert_eq!(args(vec!["--layout", "Grid"]).layout_type, LayoutType::Grid);
    }

    #[test]
    fn allows_to_change_to_the_midi_backend() {
        assert_eq!(args(vec!["--midi"]).midi, true);
    }

    #[test]
    fn allows_to_enable_dev_mode() {
        assert_eq!(args(vec!["--dev-mode"]).dev_mode, true);
    }

    #[test]
    fn allows_to_specify_harmonics() {
        assert_eq!(
            args(vec!["--harmonics", "0.1,0.2"]).harmonics_state,
            HarmonicsState {
                harmonics: [0.1, 0.2, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
            }
        );
    }

    #[test]
    fn allows_to_specify_harmonics_as_integers() {
        assert_eq!(
            args(vec!["--harmonics", "1,0,1"]).harmonics_state,
            HarmonicsState {
                harmonics: [1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0]
            }
        );
    }
}
