use crate::utils;
use crate::utils::Slots;
use crate::AddMessage;
use crate::ErrorString;
use ::evdev_rs::enums::{EventCode, EventType::*, EV_ABS, EV_SYN::*};
use ::evdev_rs::{Device, GrabMode, InputEvent, ReadFlag, ReadStatus};
use ::std::fs::File;
use ::std::iter::Flatten;

pub struct InputEventSource {
    device: Device,
}

impl InputEventSource {
    pub fn new(path: &str) -> Result<InputEventSource, ErrorString> {
        let file = File::open(path).add_message(format!("file not found: {}", path))?;
        let mut device = Device::new().ok_or("evdev: can't initialize device")?;
        device
            .set_fd(file)
            .add_message(format!("set_fd failed on {}", path))?;
        device.grab(GrabMode::Grab)?;
        Ok(InputEventSource { device })
    }
}

impl Iterator for InputEventSource {
    type Item = InputEvent;

    fn next(&mut self) -> Option<InputEvent> {
        match self
            .device
            .next_event(ReadFlag::NORMAL | ReadFlag::BLOCKING)
        {
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

pub struct SynChunkSource {
    input_event_source: Box<dyn Iterator<Item = InputEvent>>,
}

impl SynChunkSource {
    pub fn new(input_event_source: impl Iterator<Item = InputEvent> + 'static) -> SynChunkSource {
        SynChunkSource {
            input_event_source: Box::new(input_event_source),
        }
    }
}

impl ::std::fmt::Debug for SynChunkSource {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "<SynChunkSource>")
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

impl Iterator for SynChunkSource {
    type Item = Vec<InputEvent>;

    fn next(&mut self) -> Option<Vec<InputEvent>> {
        let mut result = vec![];
        loop {
            match self.input_event_source.next() {
                None => {
                    if result.is_empty() {
                        return None;
                    } else {
                        break;
                    }
                }
                Some(event) => {
                    if is_syn_dropped_event(&event) {
                        eprintln!("SynChunkSource: dropped events");
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

#[derive(Debug, Clone, Copy)]
struct SlotState {
    position: Position,
    btn_touch: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TouchState {
    NoTouch { slot: usize },
    Touch { slot: usize, position: Position },
}

#[derive(Debug)]
struct TouchStateChunkSource {
    syn_chunk_source: SynChunkSource,
    slots: Slots<SlotState>,
    active_slot: Option<usize>,
}

impl TouchStateChunkSource {
    fn from_syn_chunk_source(syn_chunk_source: SynChunkSource) -> TouchStateChunkSource {
        TouchStateChunkSource {
            syn_chunk_source,
            slots: [SlotState {
                position: Position { x: 0, y: 0 },
                btn_touch: false,
            }; 10],
            active_slot: None,
        }
    }
}

impl Iterator for TouchStateChunkSource {
    type Item = Vec<TouchState>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.syn_chunk_source.next() {
            None => None,
            Some(chunk) => {
                let mut changed = [false; 10];
                for event in chunk {
                    if let EV_ABS = event.event_type {
                        match event.event_code {
                            EventCode::EV_ABS(EV_ABS::ABS_MT_SLOT) => {
                                self.active_slot = if event.value < self.slots.as_ref().len() as i32
                                {
                                    Some(event.value as usize)
                                } else {
                                    None
                                };
                            }
                            EventCode::EV_ABS(EV_ABS::ABS_MT_POSITION_X) => {
                                if let Some(active_slot) = self.active_slot {
                                    changed[active_slot] = true;
                                    self.slots[active_slot].position.x = event.value;
                                }
                            }
                            EventCode::EV_ABS(EV_ABS::ABS_MT_POSITION_Y) => {
                                if let Some(active_slot) = self.active_slot {
                                    changed[active_slot] = true;
                                    self.slots[active_slot].position.y = event.value;
                                }
                            }
                            EventCode::EV_ABS(EV_ABS::ABS_MT_TRACKING_ID) => {
                                if let Some(active_slot) = self.active_slot {
                                    changed[active_slot] = true;
                                    match event.value {
                                        -1 => self.slots[active_slot].btn_touch = false,
                                        _ => self.slots[active_slot].btn_touch = true,
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                let mut result = vec![];
                for (slot, changed) in changed.iter().enumerate() {
                    if *changed {
                        result.push(if self.slots[slot].btn_touch {
                            TouchState::Touch {
                                slot,
                                position: self.slots[slot].position,
                            }
                        } else {
                            TouchState::NoTouch { slot }
                        })
                    }
                }
                Some(result)
            }
        }
    }
}

pub struct TouchStateSource(Flatten<TouchStateChunkSource>);

impl TouchStateSource {
    fn from_syn_chunk_source(syn_chunk_source: SynChunkSource) -> TouchStateSource {
        TouchStateSource(TouchStateChunkSource::from_syn_chunk_source(syn_chunk_source).flatten())
    }

    pub fn new(file: &str) -> Result<TouchStateSource, ErrorString> {
        Ok(TouchStateSource::from_syn_chunk_source(
            SynChunkSource::new(InputEventSource::new(file)?),
        ))
    }

    pub fn blocking() -> TouchStateSource {
        TouchStateSource::from_syn_chunk_source(SynChunkSource::new(utils::blocking()))
    }
}

impl Iterator for TouchStateSource {
    type Item = TouchState;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

#[cfg(test)]
mod test {
    use super::TouchState::*;
    use super::*;
    use ::evdev_rs::enums::{EventCode, EventType, EV_ABS::*};
    use ::evdev_rs::TimeVal;

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

        fn touch_states(vec: Vec<InputEvent>) -> TouchStateSource {
            TouchStateSource::from_syn_chunk_source(SynChunkSource::new(vec.into_iter()))
        }
    }

    mod syn_chunks {
        use super::*;

        #[test]
        fn groups_events_until_ev_syn() {
            let vec = vec![
                Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
                Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 2),
                Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            ];
            assert_eq!(
                SynChunkSource::new(vec.into_iter()).next(),
                Some(vec![
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 2),
                ])
            );
        }

        #[test]
        fn bundles_subsequent_chunks_correctly() {
            let vec = vec![
                Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
                Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                //
                Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 2),
                Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            ];
            let mut syn_chunks = SynChunkSource::new(vec.into_iter());
            syn_chunks.next();
            assert_eq!(
                syn_chunks.next(),
                Some(vec![Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 2)])
            );
        }

        #[test]
        fn handles_terminating_streams_gracefully() {
            let vec = vec![Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1)];
            let mut syn_chunks = SynChunkSource::new(vec.into_iter());
            assert_eq!(
                syn_chunks.next(),
                Some(vec![Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1)])
            );
            assert_eq!(syn_chunks.next(), None);
            assert_eq!(syn_chunks.next(), None);
        }
    }

    mod touch_states {
        use super::*;

        mod slot_zero {
            use super::*;

            #[test]
            fn relays_a_position() {
                let touch_states = Mock::touch_states(vec![
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![Touch {
                        slot: 0,
                        position: Position { x: 23, y: 42 }
                    }]
                );
            }

            #[test]
            fn relays_following_positions() {
                let touch_states = Mock::touch_states(vec![
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
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 0,
                            position: Position { x: 23, y: 42 }
                        },
                        Touch {
                            slot: 0,
                            position: Position { x: 51, y: 84 }
                        }
                    ]
                );
            }

            #[test]
            fn handles_syn_chunks_without_y() {
                let touch_states = Mock::touch_states(vec![
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 51),
                    Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 0,
                            position: Position { x: 23, y: 42 },
                        },
                        Touch {
                            slot: 0,
                            position: Position { x: 51, y: 42 }
                        },
                    ]
                );
            }

            #[test]
            fn handles_syn_chunks_without_x() {
                let touch_states = Mock::touch_states(vec![
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 84),
                    Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 0,
                            position: Position { x: 23, y: 42 },
                        },
                        Touch {
                            slot: 0,
                            position: Position { x: 23, y: 84 }
                        },
                    ]
                );
            }

            #[test]
            fn recognizes_touch_releases() {
                let touch_states = Mock::touch_states(vec![
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), -1),
                    Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 0,
                            position: Position { x: 23, y: 42 },
                        },
                        NoTouch { slot: 0 },
                    ]
                );
            }

            #[test]
            fn treats_note_on_events_in_other_slots_correctly() {
                let touch_states = Mock::touch_states(vec![
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
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 51),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 84),
                    Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 0,
                            position: Position { x: 23, y: 42 }
                        },
                        Touch {
                            slot: 1,
                            position: Position { x: 1000, y: 1000 }
                        },
                        Touch {
                            slot: 0,
                            position: Position { x: 51, y: 84 }
                        },
                    ]
                );
            }

            #[test]
            fn treats_note_off_events_in_other_slots_correctly() {
                let touch_states = Mock::touch_states(vec![
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
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 0,
                            position: Position { x: 23, y: 42 }
                        },
                        Touch {
                            slot: 1,
                            position: Position { x: 1000, y: 1000 }
                        },
                        NoTouch { slot: 1 },
                        Touch {
                            slot: 0,
                            position: Position { x: 51, y: 84 }
                        },
                    ]
                );
            }

            #[test]
            fn assumes_no_active_slot_at_startup() {
                let touch_states = Mock::touch_states(vec![
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(touch_states.collect::<Vec<TouchState>>(), vec![]);
            }

            #[test]
            fn tracks_slot_changes_and_touch_releases_in_the_same_syn_chunk_correctly() {
                let touch_states = Mock::touch_states(vec![
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
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
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 0,
                            position: Position { x: 23, y: 42 }
                        },
                        NoTouch { slot: 0 },
                        Touch {
                            slot: 1,
                            position: Position { x: 1000, y: 1000 }
                        },
                    ]
                );
            }
        }

        mod other_slots {
            use super::*;

            #[test]
            fn relays_a_position_for_other_slots() {
                let touch_states = Mock::touch_states(vec![
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![Touch {
                        slot: 1,
                        position: Position { x: 23, y: 42 }
                    }]
                );
            }

            #[test]
            fn note_off_events_contain_the_slot() {
                let touch_states = Mock::touch_states(vec![
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), -1),
                    Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 1,
                            position: Position { x: 23, y: 42 },
                        },
                        NoTouch { slot: 1 }
                    ]
                );
            }

            #[test]
            fn handles_out_of_bound_slots_gracefully() {
                let touch_states = Mock::touch_states(vec![
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1000),
                    Mock::ev(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
                    Mock::ev(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(touch_states.collect::<Vec<TouchState>>(), vec![]);
            }
        }
    }
}
