pub fn midi_to_frequency(midi: i32) -> f32 {
    440.0 * 2.0_f32.powf(((midi - 69) as f32) / 12.0)
}

pub fn frequency_to_midi(frequency: f32) -> u8 {
    (((12.0 * (frequency / 440.0).log2()).round() as i16) + 69) as u8
}

#[cfg(test)]
mod test {
    use super::*;

    mod midi_to_frequency {
        use super::*;

        #[test]
        fn converts_the_concert_pitch_correctly() {
            assert_eq!(midi_to_frequency(69), 440.0);
        }

        #[test]
        fn converts_octaves_correctly() {
            assert_eq!(midi_to_frequency(57), 220.0);
        }

        #[test]
        fn converts_semitones_correctly() {
            assert_eq!(midi_to_frequency(70), 440.0 * 2.0_f32.powf(1.0 / 12.0));
        }
    }

    mod frequency_to_midi {
        use super::*;

        #[test]
        fn converts_the_concert_pitch_correctly() {
            assert_eq!(frequency_to_midi(440.0), 69);
        }

        #[test]
        fn converts_the_middle_c_correctly() {
            assert_eq!(frequency_to_midi(261.625565), 60);
        }

        #[test]
        fn rounds_to_the_nearest_midi_pitch() {
            assert_eq!(frequency_to_midi(269.0), 60);
            assert_eq!(frequency_to_midi(270.0), 61);
        }
    }
}
