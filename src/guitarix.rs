extern crate tempdir;

use self::tempdir::TempDir;
use get_binary_name;
use quit::Quitter;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use ErrorString;

struct ConfigFile {
    temp_dir: TempDir,
}

impl ConfigFile {
    fn new() -> Result<ConfigFile, ErrorString> {
        let result = ConfigFile {
            temp_dir: TempDir::new(&get_binary_name()?)?,
        };
        let default_config = include_str!("../guitarix-config.json");
        let mut handle = File::create(result.path())?;
        handle.write_all(default_config.as_bytes())?;
        Ok(result)
    }

    fn path(&self) -> PathBuf {
        self.temp_dir
            .path()
            .join(Path::new("guitarix-config.json").to_path_buf())
    }
}

pub struct Guitarix {
    child: Child,
    _config_file: ConfigFile,
}

impl Guitarix {
    pub fn run(mut quitter: Quitter) -> Result<(), ErrorString> {
        let mut guitarix: Guitarix = Guitarix::new()?;
        quitter.register_cleanup(move || {
            guitarix.stop();
        });
        Ok(())
    }

    fn args(config_file: &ConfigFile) -> Vec<String> {
        vec![
            "--jack-input",
            "touchscreen-instrument:left-output",
            "--jack-output",
            "system:playback_1",
            "--jack-output",
            "system:playback_2",
            "--disable-save-on-exit",
            "--load-file",
            config_file.path().to_str().unwrap(),
        ]
        .into_iter()
        .map(|x| x.to_string())
        .collect()
    }

    fn new() -> Result<Guitarix, ErrorString> {
        let config_file = ConfigFile::new()?;
        let mut command = Command::new("guitarix");
        for arg in Guitarix::args(&config_file) {
            command.arg(arg);
        }
        let child = command.spawn()?;
        Ok(Guitarix {
            child,
            _config_file: config_file,
        })
    }

    fn stop(&mut self) {
        let _ = self.child.kill();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod config_file {
        use super::*;
        use std::env::{current_dir, set_current_dir};
        use std::io::prelude::*;
        use std::sync::Mutex;

        lazy_static! {
            static ref WORKING_DIRECTORY_LOCK: Mutex<()> = Mutex::new(());
        }

        fn read_file(file: PathBuf) -> String {
            let mut f = File::open(file.clone()).expect(&*format!("File::open: {:?}", file));

            let mut contents = String::new();
            f.read_to_string(&mut contents).expect("read_to_string");
            contents
        }

        #[test]
        fn returns_the_default_config() {
            let _lock = WORKING_DIRECTORY_LOCK.lock().unwrap();
            let expected = read_file(Path::new("./guitarix-config.json").to_path_buf());
            let file = ConfigFile::new().unwrap();
            let config = read_file(file.path());
            assert_eq!(config, expected);
        }

        #[test]
        fn works_outside_of_the_git_repo() {
            let _lock = WORKING_DIRECTORY_LOCK.lock().unwrap();
            let outer_working_directory = current_dir().unwrap();
            let expected = read_file(Path::new("./guitarix-config.json").to_path_buf());
            let temp_dir = TempDir::new("touchscreen-test").expect("TempDir::new");
            set_current_dir(temp_dir.path()).expect("set_current_dir");

            let config_file = ConfigFile::new().unwrap();
            let config = read_file(config_file.path());
            assert_eq!(config, expected);
            set_current_dir(outer_working_directory).expect("set_current_dir");
        }
    }

    mod args {
        use super::*;

        #[test]
        fn returns_correct_command_line_args() {
            let config_file = ConfigFile::new().unwrap();
            let config_file_path = config_file.path();
            let mut expected = vec![];
            expected.append(&mut vec![
                "--jack-input",
                "touchscreen-instrument:left-output",
            ]);
            expected.append(&mut vec!["--jack-output", "system:playback_1"]);
            expected.append(&mut vec!["--jack-output", "system:playback_2"]);
            expected.append(&mut vec!["--disable-save-on-exit"]);
            expected.append(&mut vec!["--load-file", config_file_path.to_str().unwrap()]);
            assert_eq!(Guitarix::args(&config_file), expected);
        }
    }
}
