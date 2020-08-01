use crate::areas::Areas;
use crate::evdev::{Foo, TrackedTouchState};
use crate::sound::{mk_voices, NoteEvent, POLYPHONY};

pub struct NoteEventSource {
    areas: Areas,
    touch_state_source: Box<dyn Iterator<Item = TrackedTouchState>>,
    state: [NoteEvent; POLYPHONY],
}

impl NoteEventSource {
    pub fn new(
        areas: Areas,
        touch_state_source: impl Iterator<Item = TrackedTouchState> + 'static,
    ) -> NoteEventSource {
        NoteEventSource {
            areas,
            touch_state_source: Box::new(touch_state_source),
            state: mk_voices(NoteEvent::NoteOff),
        }
    }
}

impl Iterator for NoteEventSource {
    type Item = [NoteEvent; POLYPHONY];

    fn next(&mut self) -> Option<Self::Item> {
        self.touch_state_source.next().map(|touchstate| {
            let (tracking_id, note_event) = match touchstate {
                TrackedTouchState {
                    tracking_id,
                    touch_state: Foo::NoTouch,
                } => (tracking_id, NoteEvent::NoteOff),
                TrackedTouchState {
                    tracking_id,
                    touch_state: Foo::Touch { position },
                } => (
                    tracking_id,
                    match self.areas.frequency(&position) {
                        Some(frequency) => NoteEvent::NoteOn(frequency),
                        None => NoteEvent::NoteOff,
                    },
                ),
            };
            self.state[(tracking_id % POLYPHONY as i32) as usize] = note_event;
            self.state.clone()
        })
    }
}

#[cfg(test)]
pub mod test {
    use super::NoteEvent::*;
    use super::*;
    use crate::evdev::Position;
    use crate::sound::midi::midi_to_frequency;
    use crate::sound::test::mk_test_voices;

    mod note_event_source {
        use super::*;
        use crate::areas::{AreasConfig, Orientation};

        fn areas(start_midi_note: i32) -> Areas {
            Areas::new(AreasConfig {
                touch_width: 800,
                touch_height: 600,
                orientation: Orientation::Portrait,
                u: Position { x: -0, y: -10 },
                v: Position { x: -6, y: -6 },
                column_range: (-1, 60),
                row_range: (0, 134),
                start_midi_note,
                row_interval: 7,
            })
        }

        #[test]
        fn yields_frequencies() {
            let mut frequencies = NoteEventSource::new(
                areas(48),
                vec![TrackedTouchState {
                    tracking_id: 0,
                    touch_state: Foo::Touch {
                        position: Position { x: 798, y: 595 },
                    },
                }]
                .into_iter(),
            );
            assert_eq!(
                frequencies.next().unwrap()[0],
                NoteOn(midi_to_frequency(48))
            );
        }

        #[test]
        fn yields_notouch_for_pauses() {
            let mut frequencies = NoteEventSource::new(
                areas(48),
                vec![TrackedTouchState {
                    tracking_id: 0,
                    touch_state: Foo::NoTouch,
                }]
                .into_iter(),
            );
            assert_eq!(frequencies.next().unwrap()[0], NoteOff);
        }

        #[test]
        fn allows_to_specify_the_starting_note() {
            let mut frequencies = NoteEventSource::new(
                areas(49),
                vec![TrackedTouchState {
                    tracking_id: 0,
                    touch_state: Foo::Touch {
                        position: Position { x: 798, y: 595 },
                    },
                }]
                .into_iter(),
            );
            assert_eq!(
                frequencies.next().unwrap()[0],
                NoteOn(midi_to_frequency(49))
            );
        }

        #[test]
        fn uses_the_tracking_id_as_voice_index_if_possible() {
            for i in 0..POLYPHONY {
                println!("i: {}", i);
                let mut frequencies = NoteEventSource::new(
                    areas(48),
                    vec![TrackedTouchState {
                        tracking_id: i as i32,
                        touch_state: Foo::Touch {
                            position: Position { x: 798, y: 595 },
                        },
                    }]
                    .into_iter(),
                );
                assert_eq!(
                    frequencies.next(),
                    Some(mk_test_voices(vec![(i, NoteOn(midi_to_frequency(48)))]))
                );
            }
        }

        #[test]
        fn wraps_around_when_tracking_numbers_get_too_big() {
            for tracking_id in (POLYPHONY as i32)..(POLYPHONY as i32 * 3) {
                println!("tracking_id: {}", tracking_id);
                let mut frequencies = NoteEventSource::new(
                    areas(48),
                    vec![TrackedTouchState {
                        tracking_id,
                        touch_state: Foo::Touch {
                            position: Position { x: 798, y: 595 },
                        },
                    }]
                    .into_iter(),
                );
                assert_eq!(
                    frequencies.next(),
                    Some(mk_test_voices(vec![(
                        (tracking_id % (POLYPHONY as i32)) as usize,
                        NoteOn(midi_to_frequency(48))
                    )]))
                );
            }
        }

        #[test]
        fn preserves_the_state_of_voices() {
            let mut frequencies = NoteEventSource::new(
                areas(48),
                vec![
                    TrackedTouchState {
                        tracking_id: 0,
                        touch_state: Foo::Touch {
                            position: Position { x: 798, y: 595 },
                        },
                    },
                    TrackedTouchState {
                        tracking_id: 1,
                        touch_state: Foo::Touch {
                            position: Position { x: 798, y: 595 },
                        },
                    },
                    TrackedTouchState {
                        tracking_id: 0,
                        touch_state: Foo::NoTouch,
                    },
                ]
                .into_iter(),
            );
            frequencies.next();
            assert_eq!(
                frequencies.next(),
                Some(mk_test_voices(vec![
                    (0, NoteOn(midi_to_frequency(48))),
                    (1, NoteOn(midi_to_frequency(48)))
                ]))
            );
            assert_eq!(
                frequencies.next(),
                Some(mk_test_voices(vec![(1, NoteOn(midi_to_frequency(48)))]))
            );
        }
    }
}
