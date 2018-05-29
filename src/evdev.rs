extern crate evdev_rs;

use self::evdev_rs::*;
use std::fs::File;
use AppError;

pub struct Events {
    _file: File,
    device: Device,
}

impl Events {
    pub fn new(file: &str) -> Result<Events, AppError> {
        let file = File::open(file)?;
        let mut device =
            Device::new().ok_or(AppError::new("evdev: can't initialize device".to_string()))?;
        device.set_fd(&file)?;
        device.grab(GrabMode::Grab)?;
        Ok(Events {
            _file: file,
            device,
        })
    }
}

impl Iterator for Events {
    type Item = InputEvent;

    fn next(&mut self) -> Option<InputEvent> {
        match self.device.next_event(NORMAL | BLOCKING) {
            Err(e) => {
                eprintln!("error: next: {:?}", e);
                self.next()
            }
            Ok((status, event)) => {
                if status == ReadStatus::Sync {
                    eprintln!("ReadStatus == Sync");
                }
                Some(event)
            }
        }
    }
}
