use crate::areas::Areas;
use crate::evdev::TouchState;
use crate::sound::NoteEvent;

pub struct NoteEventSource {
    areas: Areas,
    touch_state_source: Box<dyn Iterator<Item = TouchState>>,
}

impl NoteEventSource {
    pub fn new(
        areas: Areas,
        touch_state_source: impl Iterator<Item = TouchState> + 'static,
    ) -> NoteEventSource {
        NoteEventSource {
            areas,
            touch_state_source: Box::new(touch_state_source),
        }
    }
}

impl Iterator for NoteEventSource {
    type Item = NoteEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.touch_state_source
            .next()
            .map(|touchstate| match touchstate {
                TouchState::NoTouch { slot, .. } => NoteEvent::NoteOff { slot },
                TouchState::Touch { position, slot, .. } => match self.areas.frequency(position) {
                    Some(frequency) => NoteEvent::NoteOn { slot, frequency },
                    None => NoteEvent::NoteOff { slot },
                },
            })
    }
}

#[cfg(test)]
pub mod test {
    use super::NoteEvent::*;
    use super::*;
    use crate::evdev::Position;
    use crate::sound::midi::midi_to_frequency;

    mod note_event_source {
        use super::*;
        use crate::areas::{AreasConfig, Orientation};

        #[test]
        fn yields_frequencies() {
            let areas = Areas::new(AreasConfig {
                touch_width: 800,
                touch_height: 600,
                orientation: Orientation::Portrait,
                u: Position { x: -0, y: -10 },
                v: Position { x: -6, y: -6 },
                column_range: (-1, 60),
                row_range: (0, 134),
                start_midi_note: 48,
                row_interval: 7,
            });
            let mut frequencies = NoteEventSource::new(
                areas,
                vec![TouchState::Touch {
                    slot: 0,
                    tracking_id: 0,
                    position: Position { x: 798, y: 595 },
                }]
                .into_iter(),
            );
            assert_eq!(
                frequencies.next(),
                Some(NoteOn {
                    slot: 0,
                    frequency: midi_to_frequency(48)
                })
            );
        }

        #[test]
        fn yields_notouch_for_pauses() {
            let areas = Areas::new(AreasConfig {
                touch_width: 800,
                touch_height: 600,
                orientation: Orientation::Portrait,
                u: Position { x: -0, y: -10 },
                v: Position { x: -6, y: -6 },
                column_range: (-1, 60),
                row_range: (0, 134),
                start_midi_note: 48,
                row_interval: 7,
            });
            let mut frequencies = NoteEventSource::new(
                areas,
                vec![TouchState::NoTouch {
                    slot: 0,
                    tracking_id: 0,
                }]
                .into_iter(),
            );
            assert_eq!(frequencies.next(), Some(NoteOff { slot: 0 }));
        }

        #[test]
        fn allows_to_specify_the_starting_note() {
            let areas = Areas::new(AreasConfig {
                touch_width: 800,
                touch_height: 600,
                orientation: Orientation::Portrait,
                u: Position { x: -0, y: -10 },
                v: Position { x: -6, y: -6 },
                column_range: (-1, 60),
                row_range: (0, 134),
                start_midi_note: 49,
                row_interval: 7,
            });
            let mut frequencies = NoteEventSource::new(
                areas,
                vec![TouchState::Touch {
                    slot: 0,
                    tracking_id: 0,
                    position: Position { x: 798, y: 595 },
                }]
                .into_iter(),
            );
            assert_eq!(
                frequencies.next(),
                Some(NoteOn {
                    slot: 0,
                    frequency: midi_to_frequency(49)
                })
            );
        }
    }
}
