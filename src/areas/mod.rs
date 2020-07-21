pub mod layouts;
pub mod note_event_source;
pub mod render;
pub mod shape;

use crate::evdev::Position;
use crate::sound::midi::midi_to_frequency;
use sdl2::pixels::Color;
use shape::Shape;

#[derive(Clone, Debug, PartialEq)]
struct Area {
    shape: Shape,
    color: Color,
    midi_note: i32,
}

impl Area {
    fn new(shape: Shape, midi_note: i32) -> Area {
        Area {
            shape,
            color: Areas::make_color(midi_note),
            midi_note,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Areas {
    areas: Vec<Area>,
    touch_width: i32,
    touch_height: i32,
}

pub struct AreasConfig {
    pub touch_width: i32,
    pub touch_height: i32,
    pub orientation: Orientation,
    pub u: Position,
    pub v: Position,
    pub column_range: (i32, i32),
    pub row_range: (i32, i32),
    pub start_midi_note: i32,
    pub row_interval: i32,
}

pub enum Orientation {
    Portrait,
    Landscape,
}

impl Areas {
    pub fn new(
        AreasConfig {
            touch_width,
            touch_height,
            orientation,
            u,
            v,
            column_range,
            row_range,
            start_midi_note,
            row_interval,
        }: AreasConfig,
    ) -> Areas {
        let anchor = Position {
            x: match orientation {
                Orientation::Portrait => touch_width,
                Orientation::Landscape => 0,
            },
            y: touch_height,
        };
        let mut areas = vec![];
        for row in row_range.0..row_range.1 {
            for column in column_range.0..column_range.1 {
                areas.push(Area::new(
                    Shape::Parallelogram {
                        base: Position {
                            x: anchor.x + v.x * row + u.x * column,
                            y: anchor.y + u.y * column + v.y * row,
                        },
                        u,
                        v,
                    },
                    start_midi_note + column + row * row_interval,
                ));
            }
        }
        Areas {
            areas,
            touch_width,
            touch_height,
        }
    }

    pub fn frequency(&self, position: Position) -> Option<f32> {
        self.areas
            .iter()
            .find(|area| area.shape.contains(position))
            .map(|area| midi_to_frequency(area.midi_note))
    }

    fn make_color(midi_note: i32) -> Color {
        use palette::rgb::Rgb;
        use palette::rgb::Srgb;
        use palette::Hsv;

        let hue_number = (midi_note * 7) % 12;

        let c: Rgb<_, u8> =
            Srgb::from(Hsv::new(hue_number as f32 * 30.0 + 240.0, 1.0, 1.0)).into_format();
        Areas::convert_color(c)
    }

    fn convert_color(color: palette::rgb::Rgb<palette::encoding::srgb::Srgb, u8>) -> Color {
        Color::RGB(color.red, color.green, color.blue)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod areas {
        use super::*;

        mod make_color {
            use super::*;

            #[test]
            fn returns_blue_for_the_middle_c() {
                assert_eq!(Areas::make_color(60), Color::RGB(0, 0, 255));
            }

            #[test]
            fn returns_blue_one_octave_higher() {
                assert_eq!(Areas::make_color(72), Color::RGB(0, 0, 255));
            }

            #[test]
            fn cycles_through_twelve_colors_by_hue_in_cycle_of_fifth() {
                use palette::Hsv;
                use palette::Srgb;

                let mut color = Hsv::from(Srgb::new(0.0, 0.0, 1.0));
                color.hue = color.hue + 360.0 / 12.0;
                assert_eq!(
                    Areas::make_color(7),
                    Areas::convert_color(Srgb::from(color).into_format())
                );
                color.hue = color.hue + 360.0 / 12.0;
                assert_eq!(
                    Areas::make_color(62),
                    Areas::convert_color(Srgb::from(color).into_format())
                );
            }
        }

        mod new {
            use super::*;

            #[test]
            fn renders_a_parallelogram_in_the_bottom_corner() {
                let areas = Areas::new(AreasConfig {
                    touch_width: 800,
                    touch_height: 600,
                    orientation: Orientation::Portrait,
                    u: Position { x: -0, y: -10 },
                    v: Position { x: -6, y: -6 },
                    column_range: (-1, 60),
                    row_range: (0, 134),
                    start_midi_note: 36,
                    row_interval: 7,
                })
                .areas;
                assert_eq!(
                    areas[1].shape,
                    Shape::Parallelogram {
                        base: Position { x: 800, y: 600 },
                        u: Position { x: 0, y: -10 },
                        v: Position { x: -6, y: -6 },
                    }
                );
            }

            #[test]
            fn renders_subsequent_parallelograms_in_the_bottom_row() {
                let areas = Areas::new(AreasConfig {
                    touch_width: 800,
                    touch_height: 600,
                    orientation: Orientation::Portrait,
                    u: Position { x: -0, y: -10 },
                    v: Position { x: -6, y: -6 },
                    column_range: (-1, 60),
                    row_range: (0, 134),
                    start_midi_note: 36,
                    row_interval: 7,
                })
                .areas;
                assert_eq!(
                    areas[2..4]
                        .into_iter()
                        .map(|x| &x.shape)
                        .collect::<Vec<&Shape>>(),
                    vec![
                        &Shape::Parallelogram {
                            base: Position { x: 800, y: 590 },
                            u: Position { x: 0, y: -10 },
                            v: Position { x: -6, y: -6 },
                        },
                        &Shape::Parallelogram {
                            base: Position { x: 800, y: 580 },
                            u: Position { x: 0, y: -10 },
                            v: Position { x: -6, y: -6 },
                        },
                    ]
                );
            }

            #[test]
            fn renders_edge_parallelograms_in_first_row() {
                let areas = Areas::new(AreasConfig {
                    touch_width: 800,
                    touch_height: 605,
                    orientation: Orientation::Portrait,
                    u: Position { x: -0, y: -10 },
                    v: Position { x: -6, y: -6 },
                    column_range: (-1, 61),
                    row_range: (0, 134),
                    start_midi_note: 36,
                    row_interval: 7,
                })
                .areas;
                assert_eq!(
                    areas[61].shape,
                    Shape::Parallelogram {
                        base: Position { x: 800, y: 5 },
                        u: Position { x: 0, y: -10 },
                        v: Position { x: -6, y: -6 },
                    }
                );
            }

            #[test]
            fn first_row_is_chromatic_scale() {
                let areas = Areas::new(AreasConfig {
                    touch_width: 800,
                    touch_height: 600,
                    orientation: Orientation::Portrait,
                    u: Position { x: -0, y: -10 },
                    v: Position { x: -6, y: -6 },
                    column_range: (-1, 60),
                    row_range: (0, 134),
                    start_midi_note: 36,
                    row_interval: 7,
                })
                .areas;
                assert_eq!(
                    areas[0..3]
                        .into_iter()
                        .map(|x| &x.midi_note)
                        .collect::<Vec<&i32>>(),
                    vec![&35, &36, &37]
                );
            }

            #[test]
            fn renders_second_row() {
                let areas = Areas::new(AreasConfig {
                    touch_width: 800,
                    touch_height: 600,
                    orientation: Orientation::Portrait,
                    u: Position { x: -0, y: -10 },
                    v: Position { x: -6, y: -6 },
                    column_range: (-1, 60),
                    row_range: (0, 134),
                    start_midi_note: 36,
                    row_interval: 7,
                })
                .areas;
                assert_eq!(
                    areas[62..64]
                        .into_iter()
                        .map(|x| &x.shape)
                        .collect::<Vec<&Shape>>(),
                    vec![
                        &Shape::Parallelogram {
                            base: Position { x: 794, y: 594 },
                            u: Position { x: 0, y: -10 },
                            v: Position { x: -6, y: -6 },
                        },
                        &Shape::Parallelogram {
                            base: Position { x: 794, y: 584 },
                            u: Position { x: 0, y: -10 },
                            v: Position { x: -6, y: -6 },
                        },
                    ]
                );
            }

            #[test]
            fn second_row_is_a_fifth_higher() {
                let areas = Areas::new(AreasConfig {
                    touch_width: 800,
                    touch_height: 600,
                    orientation: Orientation::Portrait,
                    u: Position { x: -0, y: -10 },
                    v: Position { x: -6, y: -6 },
                    column_range: (-1, 60),
                    row_range: (0, 134),
                    start_midi_note: 36,
                    row_interval: 7,
                })
                .areas;
                assert_eq!(
                    areas[62..65]
                        .into_iter()
                        .map(|x| &x.midi_note)
                        .collect::<Vec<&i32>>(),
                    vec![&43, &44, &45]
                );
            }

            #[test]
            fn allows_to_change_interval_between_rows() {
                let areas = Areas::new(AreasConfig {
                    touch_width: 800,
                    touch_height: 600,
                    orientation: Orientation::Portrait,
                    u: Position { x: -0, y: -10 },
                    v: Position { x: -6, y: -6 },
                    column_range: (-1, 60),
                    row_range: (0, 134),
                    start_midi_note: 36,
                    row_interval: 4,
                })
                .areas;
                assert_eq!(
                    areas[62..65]
                        .into_iter()
                        .map(|x| &x.midi_note)
                        .collect::<Vec<&i32>>(),
                    vec![&40, &41, &42]
                );
            }

            #[test]
            fn allows_to_change_the_base_midi_note() {
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
                })
                .areas;
                assert_eq!(
                    areas[0..3]
                        .into_iter()
                        .map(|x| &x.midi_note)
                        .collect::<Vec<&i32>>(),
                    vec![&47, &48, &49]
                );
            }

            #[test]
            fn allows_to_configure_parallelogram_slanting() {
                let areas = Areas::new(AreasConfig {
                    touch_width: 800,
                    touch_height: 600,
                    orientation: Orientation::Portrait,
                    u: Position { x: -0, y: -10 },
                    v: Position { x: -6, y: -3 },
                    column_range: (-1, 60),
                    row_range: (0, 134),
                    start_midi_note: 36,
                    row_interval: 4,
                })
                .areas;
                assert_eq!(
                    areas[1].shape,
                    Shape::Parallelogram {
                        base: Position { x: 800, y: 600 },
                        u: Position { x: 0, y: -10 },
                        v: Position { x: -6, y: -3 },
                    }
                );
                assert_eq!(
                    areas[62].shape,
                    Shape::Parallelogram {
                        base: Position { x: 794, y: 597 },
                        u: Position { x: 0, y: -10 },
                        v: Position { x: -6, y: -3 },
                    }
                );
            }

            #[test]
            fn allows_to_configure_parallelogram_width() {
                let areas = Areas::new(AreasConfig {
                    touch_width: 800,
                    touch_height: 600,
                    orientation: Orientation::Portrait,
                    u: Position { x: -0, y: -10 },
                    v: Position { x: -10, y: -6 },
                    column_range: (-1, 60),
                    row_range: (0, 80),
                    start_midi_note: 36,
                    row_interval: 4,
                })
                .areas;
                assert_eq!(
                    areas[1].shape,
                    Shape::Parallelogram {
                        base: Position { x: 800, y: 600 },
                        u: Position { x: 0, y: -10 },
                        v: Position { x: -10, y: -6 },
                    }
                );
                assert_eq!(
                    areas[62].shape,
                    Shape::Parallelogram {
                        base: Position { x: 790, y: 594 },
                        u: Position { x: 0, y: -10 },
                        v: Position { x: -10, y: -6 },
                    }
                );
            }

            #[test]
            fn works_with_non_zero_row_slants() {
                let areas = Areas::new(AreasConfig {
                    touch_width: 800,
                    touch_height: 600,
                    orientation: Orientation::Portrait,
                    u: Position { x: -5, y: -10 },
                    v: Position { x: -6, y: -6 },
                    column_range: (-1, 60),
                    row_range: (0, 134),
                    start_midi_note: 36,
                    row_interval: 7,
                })
                .areas;
                assert_eq!(
                    areas[1].shape,
                    Shape::Parallelogram {
                        base: Position { x: 800, y: 600 },
                        u: Position { x: -5, y: -10 },
                        v: Position { x: -6, y: -6 },
                    }
                );
                assert_eq!(
                    areas[2].shape,
                    Shape::Parallelogram {
                        base: Position { x: 795, y: 590 },
                        u: Position { x: -5, y: -10 },
                        v: Position { x: -6, y: -6 },
                    }
                );
                assert_eq!(
                    areas[3].shape,
                    Shape::Parallelogram {
                        base: Position { x: 790, y: 580 },
                        u: Position { x: -5, y: -10 },
                        v: Position { x: -6, y: -6 },
                    }
                );
            }
        }
    }
}
