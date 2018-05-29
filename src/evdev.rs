extern crate evdev_rs;

use evdev::evdev_rs::enums::EventCode;
use evdev::evdev_rs::enums::EventType::*;
use evdev::evdev_rs::enums::EV_SYN::*;
use evdev::evdev_rs::*;
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

pub struct SynChunks {
    iterator: Box<Iterator<Item = InputEvent>>,
}

impl SynChunks {
    pub fn new(iterator: impl Iterator<Item = InputEvent> + 'static) -> SynChunks {
        SynChunks {
            iterator: Box::new(iterator),
        }
    }
}

fn is_syn_dropped_event(event: &InputEvent) -> bool {
    match event.event_type {
        EV_SYN => match event.event_code {
            EventCode::EV_SYN(SYN_DROPPED) => true,
            _ => false,
        },
        _ => false,
    }
}

fn is_syn_report_event(event: &InputEvent) -> bool {
    match event.event_type {
        EV_SYN => match event.event_code {
            EventCode::EV_SYN(SYN_REPORT) => true,
            _ => false,
        },
        _ => false,
    }
}

impl Iterator for SynChunks {
    type Item = Vec<InputEvent>;

    fn next(&mut self) -> Option<Vec<InputEvent>> {
        let mut result = vec![];
        loop {
            match self.iterator.next() {
                None => {
                    if result.is_empty() {
                        return None;
                    } else {
                        break;
                    }
                }
                Some(event) => {
                    if is_syn_dropped_event(&event) {
                        eprintln!("SynChunks: dropped events");
                    } else if is_syn_report_event(&event) {
                        break;
                    } else {
                        result.push(event);
                    }
                }
            }
        }
        Some(result)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use evdev::evdev_rs::enums::EventCode;
    use evdev::evdev_rs::enums::EV_ABS::*;

    fn some_event(x: i32) -> InputEvent {
        InputEvent {
            time: TimeVal {
                tv_sec: 0,
                tv_usec: 0,
            },
            event_type: EV_ABS,
            event_code: EventCode::EV_ABS(ABS_X),
            value: x,
        }
    }

    fn syn_event() -> InputEvent {
        InputEvent {
            time: TimeVal {
                tv_sec: 0,
                tv_usec: 0,
            },
            event_type: EV_SYN,
            event_code: EventCode::EV_SYN(SYN_REPORT),
            value: 0,
        }
    }

    #[test]
    fn syn_chunks_groups_events_until_ev_syn() {
        let vec = vec![some_event(1), some_event(2), syn_event()];
        assert_eq!(
            SynChunks::new(vec.into_iter()).next(),
            Some(vec![some_event(1), some_event(2)])
        );
    }

    #[test]
    fn syn_chunks_bundles_subsequent_chunks_correctly() {
        let vec = vec![some_event(1), syn_event(), some_event(2), syn_event()];
        let mut syn_chunks = SynChunks::new(vec.into_iter());
        syn_chunks.next();
        assert_eq!(syn_chunks.next(), Some(vec![some_event(2)]));
    }

    #[test]
    fn syn_chunks_handles_terminating_streams_gracefully() {
        let vec = vec![some_event(1)];
        let mut syn_chunks = SynChunks::new(vec.into_iter());
        assert_eq!(syn_chunks.next(), Some(vec![some_event(1)]));
        assert_eq!(syn_chunks.next(), None);
        assert_eq!(syn_chunks.next(), None);
    }
}
