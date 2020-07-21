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
    tracking_id: i32,
    position: Position,
    btn_touch: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TouchState {
    NoTouch {
        slot: usize,
        tracking_id: i32,
    },
    Touch {
        slot: usize,
        tracking_id: i32,
        position: Position,
    },
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
                tracking_id: 0,
                position: Position { x: 0, y: 0 },
                btn_touch: false,
            }; 10],
            active_slot: None,
        }
    }

    fn process_chunk(&mut self, chunk: Vec<InputEvent>) -> Slots<bool> {
        let mut changed = [false; 10];
        for event in chunk {
            if let EV_ABS = event.event_type {
                match event.event_code {
                    EventCode::EV_ABS(EV_ABS::ABS_MT_SLOT) => {
                        self.active_slot = if event.value < self.slots.as_ref().len() as i32 {
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
                                -1 => {
                                    self.slots[active_slot].btn_touch = false;
                                }
                                tracking_id => {
                                    self.slots[active_slot].tracking_id = tracking_id;
                                    self.slots[active_slot].btn_touch = true;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        changed
    }

    fn get_touch_state_chunk(&self, changed: Slots<bool>) -> Vec<TouchState> {
        let mut result = vec![];
        for (slot, changed) in changed.iter().enumerate() {
            if *changed {
                let slot_state = self.slots[slot];
                let touch_state = if slot_state.btn_touch {
                    TouchState::Touch {
                        slot,
                        tracking_id: slot_state.tracking_id,
                        position: slot_state.position,
                    }
                } else {
                    TouchState::NoTouch {
                        slot,
                        tracking_id: slot_state.tracking_id,
                    }
                };
                result.push(touch_state)
            }
        }
        result
    }
}

impl Iterator for TouchStateChunkSource {
    type Item = Vec<TouchState>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.syn_chunk_source.next() {
            None => None,
            Some(chunk) => {
                let changed = self.process_chunk(chunk);
                Some(self.get_touch_state_chunk(changed))
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

    fn mk_input_event(event_type: EventType, event_code: EventCode, value: i32) -> InputEvent {
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

    mod syn_chunks {
        use super::*;

        #[test]
        fn groups_events_until_ev_syn() {
            let vec = vec![
                mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
                mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 2),
                mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            ];
            assert_eq!(
                SynChunkSource::new(vec.into_iter()).next(),
                Some(vec![
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 2),
                ])
            );
        }

        #[test]
        fn bundles_subsequent_chunks_correctly() {
            let vec = vec![
                mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
                mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                //
                mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 2),
                mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
            ];
            let mut syn_chunks = SynChunkSource::new(vec.into_iter());
            syn_chunks.next();
            assert_eq!(
                syn_chunks.next(),
                Some(vec![mk_input_event(
                    EV_ABS,
                    EventCode::EV_ABS(ABS_MT_SLOT),
                    2
                )])
            );
        }

        #[test]
        fn handles_terminating_streams_gracefully() {
            let vec = vec![mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1)];
            let mut syn_chunks = SynChunkSource::new(vec.into_iter());
            assert_eq!(
                syn_chunks.next(),
                Some(vec![mk_input_event(
                    EV_ABS,
                    EventCode::EV_ABS(ABS_MT_SLOT),
                    1
                )])
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
                let touch_states = touch_states(vec![
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![Touch {
                        slot: 0,
                        tracking_id: 0,
                        position: Position { x: 23, y: 42 }
                    }]
                );
            }

            mod tracking_ids {
                use super::*;

                #[test]
                fn touch_events_includes_the_correct_tracking_id() {
                    let touch_states = touch_states(vec![
                        mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                        mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 42),
                        mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 0),
                        mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 0),
                        mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    ]);
                    assert_eq!(
                        touch_states.collect::<Vec<TouchState>>(),
                        vec![Touch {
                            slot: 0,
                            tracking_id: 42,
                            position: Position { x: 0, y: 0 }
                        },]
                    );
                }

                #[test]
                fn no_touch_events_includes_the_correct_tracking_id() {
                    let touch_states = touch_states(vec![
                        mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                        mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 42),
                        mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 0),
                        mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 0),
                        mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                        //
                        mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), -1),
                        mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    ]);
                    assert_eq!(
                        touch_states.collect::<Vec<TouchState>>(),
                        vec![
                            Touch {
                                slot: 0,
                                tracking_id: 42,
                                position: Position { x: 0, y: 0 },
                            },
                            NoTouch {
                                slot: 0,
                                tracking_id: 42,
                            },
                        ]
                    );
                }
            }

            #[test]
            fn relays_following_positions() {
                let touch_states = touch_states(vec![
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 51),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 84),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 0,
                            tracking_id: 0,
                            position: Position { x: 23, y: 42 }
                        },
                        Touch {
                            slot: 0,
                            tracking_id: 0,
                            position: Position { x: 51, y: 84 }
                        }
                    ]
                );
            }

            #[test]
            fn handles_syn_chunks_without_y() {
                let touch_states = touch_states(vec![
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 51),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 0,
                            tracking_id: 0,
                            position: Position { x: 23, y: 42 },
                        },
                        Touch {
                            slot: 0,
                            tracking_id: 0,
                            position: Position { x: 51, y: 42 }
                        },
                    ]
                );
            }

            #[test]
            fn handles_syn_chunks_without_x() {
                let touch_states = touch_states(vec![
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 84),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 0,
                            tracking_id: 0,
                            position: Position { x: 23, y: 42 },
                        },
                        Touch {
                            slot: 0,
                            tracking_id: 0,
                            position: Position { x: 23, y: 84 }
                        },
                    ]
                );
            }

            #[test]
            fn recognizes_touch_releases() {
                let touch_states = touch_states(vec![
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), -1),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 0,
                            tracking_id: 0,
                            position: Position { x: 23, y: 42 },
                        },
                        NoTouch {
                            slot: 0,
                            tracking_id: 0,
                        },
                    ]
                );
            }

            #[test]
            fn treats_note_on_events_in_other_slots_correctly() {
                let touch_states = touch_states(vec![
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 1000),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 1000),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 51),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 84),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 0,
                            tracking_id: 0,
                            position: Position { x: 23, y: 42 }
                        },
                        Touch {
                            slot: 1,
                            tracking_id: 1,
                            position: Position { x: 1000, y: 1000 }
                        },
                        Touch {
                            slot: 0,
                            tracking_id: 0,
                            position: Position { x: 51, y: 84 }
                        },
                    ]
                );
            }

            #[test]
            fn treats_note_off_events_in_other_slots_correctly() {
                let touch_states = touch_states(vec![
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 1000),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 1000),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), -1),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 51),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 84),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 0,
                            tracking_id: 0,
                            position: Position { x: 23, y: 42 },
                        },
                        Touch {
                            slot: 1,
                            tracking_id: 1,
                            position: Position { x: 1000, y: 1000 },
                        },
                        NoTouch {
                            slot: 1,
                            tracking_id: 1
                        },
                        Touch {
                            slot: 0,
                            tracking_id: 0,
                            position: Position { x: 51, y: 84 },
                        },
                    ]
                );
            }

            #[test]
            fn assumes_no_active_slot_at_startup() {
                let touch_states = touch_states(vec![
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(touch_states.collect::<Vec<TouchState>>(), vec![]);
            }

            #[test]
            fn tracks_slot_changes_and_touch_releases_in_the_same_syn_chunk_correctly() {
                let touch_states = touch_states(vec![
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 0),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), -1),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 1000),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 1000),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 0,
                            tracking_id: 0,
                            position: Position { x: 23, y: 42 }
                        },
                        NoTouch {
                            slot: 0,
                            tracking_id: 0,
                        },
                        Touch {
                            slot: 1,
                            tracking_id: 1,
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
                let touch_states = touch_states(vec![
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![Touch {
                        slot: 1,
                        tracking_id: 1,
                        position: Position { x: 23, y: 42 }
                    }]
                );
            }

            #[test]
            fn note_off_events_contain_the_slot() {
                let touch_states = touch_states(vec![
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 1),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_X), 23),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_POSITION_Y), 42),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                    //
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), -1),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(
                    touch_states.collect::<Vec<TouchState>>(),
                    vec![
                        Touch {
                            slot: 1,
                            tracking_id: 1,
                            position: Position { x: 23, y: 42 },
                        },
                        NoTouch {
                            slot: 1,
                            tracking_id: 1,
                        }
                    ]
                );
            }

            #[test]
            fn handles_out_of_bound_slots_gracefully() {
                let touch_states = touch_states(vec![
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_SLOT), 1000),
                    mk_input_event(EV_ABS, EventCode::EV_ABS(ABS_MT_TRACKING_ID), 0),
                    mk_input_event(EV_SYN, EventCode::EV_SYN(SYN_REPORT), 0),
                ]);
                assert_eq!(touch_states.collect::<Vec<TouchState>>(), vec![]);
            }
        }
    }
}
