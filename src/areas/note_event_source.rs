use areas::Areas;
use evdev::{slot_map, Position, Slots, TouchState};
use sound::NoteEvent;

pub struct NoteEventSource {
    areas: Areas,
    position_source: Box<Iterator<Item = Slots<TouchState<Position>>>>,
}

impl NoteEventSource {
    pub fn new(
        areas: Areas,
        position_source: impl Iterator<Item = Slots<TouchState<Position>>> + 'static,
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
    use sound::midi::midi_to_frequency;

    impl<T> Default for TouchState<T> {
        fn default() -> TouchState<T> {
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

    fn pos(x: i32) -> Position {
        Position { x, y: 5 }
    }

    mod note_event_source {
        use super::*;

        #[test]
        fn yields_frequencies() {
            let areas = Areas::stripes(800, 600, 10, 48);
            let mut frequencies =
                NoteEventSource::new(areas, mock_touches(vec![TouchState::Touch(pos(5))]));
            assert_eq!(
                frequencies.next(),
                Some(from_single(NoteOn(midi_to_frequency(48))))
            );
        }

        #[test]
        fn yields_notouch_for_pauses() {
            let areas = Areas::stripes(800, 600, 10, 48);
            let mut frequencies =
                NoteEventSource::new(areas, mock_touches(vec![TouchState::NoTouch]));
            assert_eq!(frequencies.next(), Some(from_single(NoteOff)));
        }

        #[test]
        fn allows_to_specify_the_starting_note() {
            let areas = Areas::stripes(800, 600, 10, 49);
            let mut frequencies =
                NoteEventSource::new(areas, mock_touches(vec![TouchState::Touch(pos(5))]));
            assert_eq!(
                frequencies.next(),
                Some(from_single(NoteOn(midi_to_frequency(49))))
            );
        }
    }
}
