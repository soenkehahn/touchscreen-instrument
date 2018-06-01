mod render;

use evdev::{Position, TouchState};

fn midi_to_frequency(midi: i32) -> f32 {
    440.0 * 2.0_f32.powf(((midi - 69) as f32) / 12.0)
}

#[derive(Copy, Clone)]
pub struct Areas {
    area_size: i32,
    start_midi_note: i32,
}

impl Areas {
    pub fn new(area_size: i32, start_midi_note: i32) -> Areas {
        Areas {
            area_size,
            start_midi_note,
        }
    }

    pub fn frequency(&self, position: Position) -> f32 {
        midi_to_frequency(self.start_midi_note + (position.x as f32 / self.area_size as f32) as i32)
    }
}

pub struct Frequencies {
    areas: Areas,
    iterator: Box<Iterator<Item = TouchState<Position>>>,
}

impl Frequencies {
    pub fn new(
        areas: Areas,
        iterator: impl Iterator<Item = TouchState<Position>> + 'static,
    ) -> Frequencies {
        Frequencies {
            areas,
            iterator: Box::new(iterator),
        }
    }
}

impl Iterator for Frequencies {
    type Item = TouchState<f32>;

    fn next(&mut self) -> Option<TouchState<f32>> {
        self.iterator
            .next()
            .map(|touchstate| touchstate.map(|position| self.areas.frequency(position)))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn midi_to_frequency_converts_the_concert_pitch_correctly() {
        assert_eq!(midi_to_frequency(69), 440.0);
    }

    #[test]
    fn midi_to_frequency_converts_octaves_correctly() {
        assert_eq!(midi_to_frequency(57), 220.0);
    }

    #[test]
    fn midi_to_frequency_converts_semitones_correctly() {
        assert_eq!(midi_to_frequency(70), 440.0 * 2.0_f32.powf(1.0 / 12.0));
    }

    fn pos(x: i32) -> Position {
        Position { x, y: 0 }
    }

    #[test]
    fn frequency_maps_x_values_to_frequencies() {
        let areas = Areas::new(10, 48);
        assert_eq!(areas.frequency(pos(5)), midi_to_frequency(48));
    }

    #[test]
    fn frequency_maps_higher_x_values_to_higher_frequencies() {
        let areas = Areas::new(10, 48);
        assert_eq!(areas.frequency(pos(15)), midi_to_frequency(49));
    }

    #[test]
    fn frequency_has_non_continuous_steps() {
        let areas = Areas::new(10, 48);
        assert_eq!(areas.frequency(pos(9)), midi_to_frequency(48));
        assert_eq!(areas.frequency(pos(10)), midi_to_frequency(49));
    }

    #[test]
    fn frequency_allows_to_change_area_size() {
        let areas = Areas::new(12, 48);
        assert_eq!(areas.frequency(pos(11)), midi_to_frequency(48));
        assert_eq!(areas.frequency(pos(12)), midi_to_frequency(49));
    }

    #[test]
    fn frequencies_yields_frequencies() {
        let areas = Areas::new(10, 48);
        let mut frequencies = Frequencies::new(areas, vec![TouchState::Touch(pos(5))].into_iter());
        assert_eq!(
            frequencies.next(),
            Some(TouchState::Touch(midi_to_frequency(48)))
        );
    }

    #[test]
    fn frequencies_yields_notouch_for_pauses() {
        let areas = Areas::new(10, 48);
        let mut frequencies = Frequencies::new(areas, vec![TouchState::NoTouch].into_iter());
        assert_eq!(frequencies.next(), Some(TouchState::NoTouch));
    }

    #[test]
    fn frequencies_allows_to_specify_the_starting_note() {
        let areas = Areas::new(10, 49);
        let mut frequencies = Frequencies::new(areas, vec![TouchState::Touch(pos(5))].into_iter());
        assert_eq!(
            frequencies.next(),
            Some(TouchState::Touch(midi_to_frequency(49)))
        );
    }
}
