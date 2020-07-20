use crate::areas::Areas;
use crate::evdev::TouchState;
use crate::sound::NoteEvent;
use crate::utils::{slot_map, Slots};

pub struct NoteEventSource {
    areas: Areas,
    touch_state_source: Box<dyn Iterator<Item = Slots<TouchState>>>,
}

impl NoteEventSource {
    pub fn new(
        areas: Areas,
        touch_state_source: impl Iterator<Item = Slots<TouchState>> + 'static,
    ) -> NoteEventSource {
        NoteEventSource {
            areas,
            touch_state_source: Box::new(touch_state_source),
        }
    }
}

impl Iterator for NoteEventSource {
    type Item = Slots<NoteEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        self.touch_state_source.next().map(|slots| {
            slot_map(slots, |touchstate| match touchstate {
                TouchState::NoTouch { slot: _ } => NoteEvent::NoteOff,
                TouchState::Touch { position, slot: _ } => self.areas.frequency(*position),
            })
        })
    }
}

#[cfg(test)]
pub mod test {
    use super::NoteEvent::*;
    use super::*;
    use crate::evdev::Position;
    use crate::sound::midi::midi_to_frequency;

    pub fn from_single_note_event(element: NoteEvent) -> Slots<NoteEvent> {
        let mut slots = [NoteEvent::default(); 10];
        slots[0] = element;
        slots
    }

    fn mock_touches(touches: Vec<TouchState>) -> impl Iterator<Item = Slots<TouchState>> {
        touches
            .into_iter()
            .map(|element: TouchState| -> Slots<TouchState> {
                let mut slots = [TouchState::NoTouch { slot: 0 }; 10];
                for (i, slot) in slots.iter_mut().enumerate() {
                    *slot = TouchState::NoTouch { slot: i };
                }
                slots[0] = element;
                slots
            })
    }

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
                mock_touches(vec![TouchState::Touch {
                    slot: 0,
                    position: Position { x: 798, y: 595 },
                }]),
            );
            assert_eq!(
                frequencies.next(),
                Some(from_single_note_event(NoteOn(midi_to_frequency(48))))
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
            let mut frequencies =
                NoteEventSource::new(areas, mock_touches(vec![TouchState::NoTouch { slot: 0 }]));
            assert_eq!(frequencies.next(), Some(from_single_note_event(NoteOff)));
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
                mock_touches(vec![TouchState::Touch {
                    slot: 0,
                    position: Position { x: 798, y: 595 },
                }]),
            );
            assert_eq!(
                frequencies.next(),
                Some(from_single_note_event(NoteOn(midi_to_frequency(49))))
            );
        }
    }
}
