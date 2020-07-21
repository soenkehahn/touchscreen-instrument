use crate::areas::Areas;
use crate::evdev::TouchState;
use crate::sound::NoteEvent;
use crate::utils::{slot_map, Slots};

pub struct NoteEventSource {
    areas: Areas,
    position_source: Box<dyn Iterator<Item = Slots<TouchState>>>,
}

impl NoteEventSource {
    pub fn new(
        areas: Areas,
        position_source: impl Iterator<Item = Slots<TouchState>> + 'static,
    ) -> NoteEventSource {
        NoteEventSource {
            areas,
            position_source: Box::new(position_source),
        }
    }
}

impl Iterator for NoteEventSource {
    type Item = Slots<NoteEvent>;

    fn next(&mut self) -> Option<Self::Item> {
        self.position_source.next().map(|slots| {
            slot_map(slots, |touchstate| match touchstate {
                TouchState::NoTouch => NoteEvent::NoteOff,
                TouchState::Touch(position) => self.areas.frequency(*position),
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

    impl Default for TouchState {
        fn default() -> TouchState {
            TouchState::NoTouch
        }
    }

    pub fn from_single<T: Copy + Default>(element: T) -> Slots<T> {
        let mut slots = [T::default(); 10];
        slots[0] = element;
        slots
    }

    fn mock_touches<T: Copy + Default>(touches: Vec<T>) -> impl Iterator<Item = Slots<T>> {
        touches.into_iter().map(from_single)
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
                mock_touches(vec![TouchState::Touch(Position { x: 798, y: 595 })]),
            );
            assert_eq!(
                frequencies.next(),
                Some(from_single(NoteOn(midi_to_frequency(48))))
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
                NoteEventSource::new(areas, mock_touches(vec![TouchState::NoTouch]));
            assert_eq!(frequencies.next(), Some(from_single(NoteOff)));
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
                mock_touches(vec![TouchState::Touch(Position { x: 798, y: 595 })]),
            );
            assert_eq!(
                frequencies.next(),
                Some(from_single(NoteOn(midi_to_frequency(49))))
            );
        }
    }
}
