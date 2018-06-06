extern crate palette;
extern crate sdl2;

pub mod render;

use self::palette::rgb::Srgb;
use evdev::{Position, TouchState};

fn midi_to_frequency(midi: i32) -> f32 {
    440.0 * 2.0_f32.powf(((midi - 69) as f32) / 12.0)
}

#[derive(Copy, Clone)]
pub struct Areas {
    area_size: i32,
    start_midi_note: i32,
    x_factor: f32,
    y_factor: f32,
}

impl Areas {
    pub fn new(area_size: i32, start_midi_note: i32, x_factor: f32, y_factor: f32) -> Areas {
        Areas {
            area_size,
            start_midi_note,
            x_factor,
            y_factor,
        }
    }

    pub fn frequency(&self, position: Position) -> f32 {
        midi_to_frequency(self.start_midi_note + (position.x as f32 / self.area_size as f32) as i32)
    }

    fn make_color(&self, i_in_cycle_of_fifths: usize) -> Color {
        use self::palette::Hsv;
        use self::palette::rgb::Rgb;

        let chromatic = (i_in_cycle_of_fifths * 7) % 12;

        let c: Rgb<_, u8> =
            Srgb::from(Hsv::new(chromatic as f32 * 30.0 + 240.0, 1.0, 1.0)).into_format();
        Srgb::new(c.red, c.green, c.blue)
    }

    fn ui_elements(&self) -> Vec<(Rectangle, Color)> {
        let mut result = vec![];
        for i in 0..30 {
            result.push((
                Rectangle {
                    x: (i as f32 * self.area_size as f32 * self.x_factor) as i32,
                    y: (1.0 * self.y_factor) as i32,
                    w: (self.area_size as f32 * self.x_factor) as i32,
                    h: (10000.0 * self.y_factor) as i32,
                },
                self.make_color(i),
            ));
        }
        result
    }
}

#[derive(PartialEq, Debug)]
struct Rectangle {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

type Color = self::palette::rgb::Rgb<palette::encoding::srgb::Srgb, u8>;

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

    fn pos(x: i32) -> Position {
        Position { x, y: 0 }
    }

    mod areas {
        use super::*;

        mod frequency {
            use super::*;

            #[test]
            fn maps_x_values_to_frequencies() {
                let areas = Areas::new(10, 48, 1.0, 1.0);
                assert_eq!(areas.frequency(pos(5)), midi_to_frequency(48));
            }

            #[test]
            fn maps_higher_x_values_to_higher_frequencies() {
                let areas = Areas::new(10, 48, 1.0, 1.0);
                assert_eq!(areas.frequency(pos(15)), midi_to_frequency(49));
            }

            #[test]
            fn has_non_continuous_steps() {
                let areas = Areas::new(10, 48, 1.0, 1.0);
                assert_eq!(areas.frequency(pos(9)), midi_to_frequency(48));
                assert_eq!(areas.frequency(pos(10)), midi_to_frequency(49));
            }

            #[test]
            fn allows_to_change_area_size() {
                let areas = Areas::new(12, 48, 1.0, 1.0);
                assert_eq!(areas.frequency(pos(11)), midi_to_frequency(48));
                assert_eq!(areas.frequency(pos(12)), midi_to_frequency(49));
            }
        }

        mod make_color {
            use super::*;

            #[test]
            fn returns_blue_as_the_first_color() {
                let areas = Areas::new(10, 48, 1.0, 1.0);
                assert_eq!(areas.make_color(0), Srgb::new(0, 0, 254));
            }

            #[test]
            fn returns_blue_one_octave_higher() {
                let areas = Areas::new(10, 48, 1.0, 1.0);
                assert_eq!(areas.make_color(12), Srgb::new(0, 0, 254));
            }

            #[test]
            fn cycles_through_twelve_colors_by_hue_in_cycle_of_fifth() {
                use self::palette::Hsv;
                use self::palette::Srgb;

                let areas = Areas::new(10, 48, 1.0, 1.0);
                let mut color = Hsv::from(Srgb::new(0.0, 0.0, 1.0));
                color.hue = color.hue + 360.0 / 12.0;
                assert_eq!(areas.make_color(7), Srgb::from(color).into_format());
                color.hue = color.hue + 360.0 / 12.0;
                assert_eq!(areas.make_color(2), Srgb::from(color).into_format());
            }
        }

        mod ui_elements {
            use super::*;

            #[test]
            fn returns_a_rectangle_for_the_lowest_pitch() {
                let elements = Areas::new(10, 48, 1.0, 1.0).ui_elements();
                assert_eq!(
                    elements.get(0).unwrap().0,
                    Rectangle {
                        x: 0,
                        y: 1,
                        w: 10,
                        h: 10000,
                    }
                );
            }

            #[test]
            fn returns_rectangles_for_higher_pitches() {
                let elements = Areas::new(10, 48, 1.0, 1.0).ui_elements();
                assert_eq!(
                    elements.get(1).unwrap().0,
                    Rectangle {
                        x: 10,
                        y: 1,
                        w: 10,
                        h: 10000,
                    }
                );
                assert_eq!(
                    elements.get(2).unwrap().0,
                    Rectangle {
                        x: 20,
                        y: 1,
                        w: 10,
                        h: 10000,
                    }
                );
            }

            #[test]
            fn translates_touch_coordinates_to_screen_coordinates() {
                let elements = Areas::new(10, 48, 0.7, 0.5).ui_elements();
                assert_eq!(
                    elements.get(2).unwrap().0,
                    Rectangle {
                        x: 14,
                        y: 0,
                        w: 7,
                        h: 5000,
                    }
                );
            }

            #[test]
            fn factors_in_the_area_size() {
                let elements = Areas::new(12, 48, 0.7, 0.5).ui_elements();
                assert_eq!(
                    elements.get(2).unwrap().0,
                    Rectangle {
                        x: (24.0 * 0.7) as i32,
                        y: (1.0 * 0.5) as i32,
                        w: (12.0 * 0.7) as i32,
                        h: (10000.0 * 0.5) as i32,
                    }
                );
            }
        }
    }

    mod frequencies {
        use super::*;

        #[test]
        fn yields_frequencies() {
            let areas = Areas::new(10, 48, 1.0, 1.0);
            let mut frequencies =
                Frequencies::new(areas, vec![TouchState::Touch(pos(5))].into_iter());
            assert_eq!(
                frequencies.next(),
                Some(TouchState::Touch(midi_to_frequency(48)))
            );
        }

        #[test]
        fn yields_notouch_for_pauses() {
            let areas = Areas::new(10, 48, 1.0, 1.0);
            let mut frequencies = Frequencies::new(areas, vec![TouchState::NoTouch].into_iter());
            assert_eq!(frequencies.next(), Some(TouchState::NoTouch));
        }

        #[test]
        fn allows_to_specify_the_starting_note() {
            let areas = Areas::new(10, 49, 1.0, 1.0);
            let mut frequencies =
                Frequencies::new(areas, vec![TouchState::Touch(pos(5))].into_iter());
            assert_eq!(
                frequencies.next(),
                Some(TouchState::Touch(midi_to_frequency(49)))
            );
        }
    }
}
