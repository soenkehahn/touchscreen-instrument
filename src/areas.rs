use input::Position;

fn midi_to_frequency(midi: i32) -> f32 {
    440.0 * 2.0_f32.powf(((midi - 69) as f32) / 12.0)
}

pub struct Areas {
    area_size: i32,
}

impl Areas {
    pub fn new(area_size: i32) -> Areas {
        Areas { area_size }
    }

    pub fn frequency(&self, position: Position) -> f32 {
        midi_to_frequency(48 + (position.x as f32 / self.area_size as f32) as i32)
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
        let areas = Areas::new(10);
        assert_eq!(areas.frequency(pos(5)), midi_to_frequency(48));
    }

    #[test]
    fn frequency_maps_higher_x_values_to_higher_frequencies() {
        let areas = Areas::new(10);
        assert_eq!(areas.frequency(pos(15)), midi_to_frequency(49));
    }

    #[test]
    fn frequency_has_non_continuous_steps() {
        let areas = Areas::new(10);
        assert_eq!(areas.frequency(pos(9)), midi_to_frequency(48));
        assert_eq!(areas.frequency(pos(10)), midi_to_frequency(49));
    }

    #[test]
    fn frequency_allows_to_change_area_size() {
        let areas = Areas::new(12);
        assert_eq!(areas.frequency(pos(11)), midi_to_frequency(48));
        assert_eq!(areas.frequency(pos(12)), midi_to_frequency(49));
    }
}
