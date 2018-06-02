extern crate evdev_rs;

use AppError;
use evdev::evdev_rs::enums::{EV_SYN::*, EventCode, EventType::*, EV_ABS};
use evdev::evdev_rs::*;
use std::fs::File;
use to_app_error;

pub struct Events {
    _file: File,
    device: Device,
}

impl Events {
    pub fn new(path: &str) -> Result<Events, AppError> {
        let file =
            File::open(path).map_err(|_| AppError::new(format!("file not found: {}", path)))?;
        let mut device = to_app_error(Device::new(), "evdev: can't initialize device")?;
        device
            .set_fd(&file)
            .map_err(|e| AppError::new(format!("set_fd failed on {} ({:?})", path, e)))?;
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

impl ::std::fmt::Debug for SynChunks {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "<SynChunks>")
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

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug)]
pub struct Positions {
    syn_chunks: SynChunks,
    position: Position,
    btn_touch: bool,
    slot_active: bool,
}

impl Positions {
    fn new_from_iterator(iterator: impl Iterator<Item = InputEvent> + 'static) -> Positions {
        Positions {
            syn_chunks: SynChunks::new(iterator),
            position: Position { x: 0, y: 0 },
            btn_touch: false,
            slot_active: true,
        }
    }

    pub fn new(file: &str) -> Result<Positions, AppError> {
        Ok(Positions::new_from_iterator(Events::new(file)?))
    }
}

#[derive(Debug, PartialEq)]
pub enum TouchState<T> {
    NoTouch,
    Touch(T),
}

impl<T> TouchState<T> {
    pub fn map<F, U>(self, f: F) -> TouchState<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            TouchState::NoTouch => TouchState::NoTouch,
            TouchState::Touch(t) => TouchState::Touch(f(t)),
        }
    }
}

impl Iterator for Positions {
    type Item = TouchState<Position>;

    fn next(&mut self) -> Option<TouchState<Position>> {
        match self.syn_chunks.next() {
            None => None,
            Some(chunk) => {
                for event in chunk {
                    match event.event_type {
                        EV_ABS => match event.event_code {
                            EventCode::EV_ABS(EV_ABS::ABS_MT_SLOT) => match event.value {
                                0 => self.slot_active = true,
                                _ => self.slot_active = false,
                            },
                            EventCode::EV_ABS(EV_ABS::ABS_MT_POSITION_X) => {
                                if self.slot_active {
                                    self.position.x = event.value;
                                }
                            }
                            EventCode::EV_ABS(EV_ABS::ABS_MT_POSITION_Y) => {
                                if self.slot_active {
                                    self.position.y = event.value;
                                }
                            }
                            EventCode::EV_ABS(EV_ABS::ABS_MT_TRACKING_ID) => if self.slot_active {
                                match event.value {
                                    -1 => self.btn_touch = false,
                                    _ => self.btn_touch = true,
                                }
                            },
                            _ => {}
                        },
                        _ => {}
                    }
                }
                Some(if self.btn_touch {
                    TouchState::Touch(self.position)
                } else {
                    TouchState::NoTouch
                })
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::TouchState::*;
    use super::*;
    use evdev::evdev_rs::enums::{EV_ABS::*, EventCode, EventType};

    struct Mock;

    impl Mock {
        fn ev(event_type: EventType, event_code: EventCode, value: i32) -> InputEvent {
            InputEvent {
                time: TimeVal {
                    tv_sec: 0,
                    tv_usec: 0,
                },
                event_type,
                event_code,
                value,
            }
        }

        fn positions(vec: Vec<InputEvent>) -> Positions {
            Positions::new_from_iterator(vec.into_iter())
        }
    }

    #[test]
    fn syn_chunks_groups_events_until_ev_syn() {
        let vec = vec![
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 2),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
        ];
        assert_eq!(
            SynChunks::new(vec.into_iter()).next(),
            Some(vec![
                Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
                Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 2),
            ])
        );
    }

    #[test]
    fn syn_chunks_bundles_subsequent_chunks_correctly() {
        let vec = vec![
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            //
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 2),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
        ];
        let mut syn_chunks = SynChunks::new(vec.into_iter());
        syn_chunks.next();
        assert_eq!(
            syn_chunks.next(),
            Some(vec![Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 2)])
        );
    }

    #[test]
    fn syn_chunks_handles_terminating_streams_gracefully() {
        let vec = vec![Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1)];
        let mut syn_chunks = SynChunks::new(vec.into_iter());
        assert_eq!(
            syn_chunks.next(),
            Some(vec![Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1)])
        );
        assert_eq!(syn_chunks.next(), None);
        assert_eq!(syn_chunks.next(), None);
    }

    // * Positions tests

    #[test]
    fn positions_relays_a_position() {
        let mut positions = Mock::positions(vec![
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
        ]);
        assert_eq!(positions.next(), Some(Touch(Position { x: 23, y: 42 })));
    }

    #[test]
    fn positions_relays_following_positions() {
        let mut positions = Mock::positions(vec![
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            //
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 51),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 84),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
        ]);
        positions.next();
        assert_eq!(positions.next(), Some(Touch(Position { x: 51, y: 84 })));
    }

    #[test]
    fn positions_handles_syn_chunks_without_y() {
        let mut positions = Mock::positions(vec![
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            //
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 51),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
        ]);
        positions.next();
        assert_eq!(positions.next(), Some(Touch(Position { x: 51, y: 42 })));
    }

    #[test]
    fn positions_handles_syn_chunks_without_x() {
        let mut positions = Mock::positions(vec![
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            //
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 84),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
        ]);
        positions.next();
        assert_eq!(positions.next(), Some(Touch(Position { x: 23, y: 84 })));
    }

    #[test]
    fn positions_recognizes_touch_releases() {
        let mut positions = Mock::positions(vec![
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            //
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), -1),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
        ]);
        positions.next();
        assert_eq!(positions.next(), Some(NoTouch));
    }

    #[test]
    fn positions_ignores_movements_from_other_slots() {
        let mut positions = Mock::positions(vec![
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            //
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 1000),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 1000),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            //
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 51),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 84),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
        ]);
        positions.next();
        assert_eq!(positions.next(), Some(Touch(Position { x: 23, y: 42 })));
        assert_eq!(positions.next(), Some(Touch(Position { x: 51, y: 84 })));
    }

    #[test]
    fn positions_ignores_touch_releases_from_other_slots() {
        let mut positions = Mock::positions(vec![
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            //
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 2),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 1000),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 1000),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            //
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), -1),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            //
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 51),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 84),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
        ]);
        positions.next();
        assert_eq!(positions.next(), Some(Touch(Position { x: 23, y: 42 })));
        assert_eq!(positions.next(), Some(Touch(Position { x: 23, y: 42 })));
        assert_eq!(positions.next(), Some(Touch(Position { x: 51, y: 84 })));
    }

    #[test]
    fn positions_assumes_slot_zero_at_start() {
        let mut positions = Mock::positions(vec![
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
        ]);
        assert_eq!(positions.next(), Some(Touch(Position { x: 23, y: 42 })));
    }

    #[test]
    fn positions_tracks_slot_changes_and_touch_releases_in_the_same_syn_chunk_correctly() {
        let mut positions = Mock::positions(vec![
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            //
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), -1),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 2),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 1000),
            Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 1000),
            Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
        ]);
        assert_eq!(positions.next(), Some(Touch(Position { x: 23, y: 42 })));
        assert_eq!(positions.next(), Some(NoTouch));
    }
}
