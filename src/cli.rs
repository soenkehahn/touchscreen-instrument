use crate::sound::wave_form::WaveFormConfig;
use crate::ErrorString;
use crate::LayoutType;
use clap::{App, Arg};
use std::ffi::OsString;

#[derive(Debug, Clone, PartialEq)]
pub struct Args {
    pub volume: f32,
    pub layout_type: LayoutType,
    pub midi: bool,
    pub wave_form_config: WaveFormConfig,
    pub dev_mode: bool,
    pub auto_connect: bool,
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
                .help("switches to the hammond sound and takes harmonics as arguments, separated by commas, e.g. '1,0.5,0.25'")
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
        ).arg(
            Arg::with_name("auto-connect")
                .long("auto-connect")
                .help("whether to connect to jack's system playback sink automatically (default: true)")
                .takes_value(true)
                .default_value("true")
                .possible_values(&["true", "false"]),
        );
    let matches = app.get_matches_from(args);
    Ok(Args {
        volume: parse_volume(matches.value_of("volume"))?,
        layout_type: parse_layout_type(matches.value_of("layout"))?,
        wave_form_config: parse_wave_form_config(matches.value_of("harmonics"))?,
        midi: matches.is_present("midi"),
        dev_mode: matches.is_present("dev-mode"),
        auto_connect: matches.value_of("auto-connect") == Some("true"),
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

fn parse_wave_form_config(input: Option<&str>) -> Result<WaveFormConfig, ErrorString> {
    match input {
        None => Ok(WaveFormConfig::Rectangle),
        Some(harmonics) => {
            let mut vector: Vec<f32> = vec![];
            for harmonic in harmonics.split(',') {
                vector.push(harmonic.parse()?)
            }
            Ok(WaveFormConfig::Harmonics(vector))
        }
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
            midi: false,
            wave_form_config: WaveFormConfig::Rectangle,
            dev_mode: false,
            auto_connect: true,
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
            args(vec!["--harmonics", "0.1,0.2"]).wave_form_config,
            WaveFormConfig::Harmonics(vec![0.1, 0.2])
        );
    }

    #[test]
    fn allows_to_specify_harmonics_as_integers() {
        assert_eq!(
            args(vec!["--harmonics", "1,0,1"]).wave_form_config,
            WaveFormConfig::Harmonics(vec![1.0, 0.0, 1.0])
        );
    }

    #[test]
    fn allows_to_disable_autoconnecting_to_system_playback() {
        assert_eq!(args(vec!["--auto-connect", "false"]).auto_connect, false);
    }
}
