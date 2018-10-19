extern crate palette;
extern crate sdl2;

pub mod note_event_source;
pub mod render;
pub mod shape;

use self::sdl2::pixels::Color;
use self::shape::Shape;
use evdev::Position;
use sound::midi::midi_to_frequency;
use sound::NoteEvent;

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

impl Areas {
    pub fn parallelograms(
        touch_width: i32,
        touch_height: i32,
        (width, height): (i32, i32),
        slantiness: i32,
        start_midi_note: i32,
        row_interval: i32,
    ) -> Areas {
        let mut areas = vec![];
        let ncols = (touch_height as f32 / height as f32).ceil() as i32;
        let nrows = (touch_width as f32 / width as f32).ceil() as i32;
        for row in 0..nrows {
            for col in -1..ncols {
                areas.push(Area::new(
                    Shape::Parallelogram {
                        base: Position {
                            x: touch_width - width * row,
                            y: touch_height - height * col - (slantiness * row),
                        },
                        u: Position {
                            x: -width,
                            y: -slantiness,
                        },
                        v: Position { x: 0, y: -height },
                    },
                    start_midi_note + col + row * row_interval,
                ));
            }
        }
        Areas {
            areas,
            touch_width,
            touch_height,
        }
    }

    pub fn flipped(
        touch_width: i32,
        touch_height: i32,
        row_length: i32,
        number_of_rows: i32,
        start_midi_note: i32,
    ) -> Areas {
        let rect_width = touch_width / row_length;
        let rect_height = touch_height / number_of_rows;
        let mut areas = vec![];
        for y in 0..number_of_rows {
            for x in 0..row_length {
                areas.push(Area::new(
                    Shape::Rectangle {
                        x: rect_width * x,
                        y: touch_height - rect_height - rect_height * y,
                        width: rect_width,
                        height: rect_height,
                    },
                    start_midi_note + x + y * 5,
                ));
            }
        }
        Areas {
            areas,
            touch_width,
            touch_height,
        }
    }

    pub fn frequency(&self, position: Position) -> NoteEvent {
        let touched: Option<&Area> = self
            .areas
            .iter()
            .filter(|area| area.shape.contains(position))
            .next();
        match touched {
            None => NoteEvent::NoteOff,
            Some(area) => NoteEvent::NoteOn(midi_to_frequency(area.midi_note)),
        }
    }

    fn make_color(midi_note: i32) -> Color {
        use self::palette::rgb::Rgb;
        use self::palette::rgb::Srgb;
        use self::palette::Hsv;

        let hue_number = (midi_note * 7) % 12;

        let c: Rgb<_, u8> =
            Srgb::from(Hsv::new(hue_number as f32 * 30.0 + 240.0, 1.0, 1.0)).into_format();
        Areas::convert_color(c)
    }

    fn convert_color(color: palette::rgb::Rgb<self::palette::encoding::srgb::Srgb, u8>) -> Color {
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
                assert_eq!(Areas::make_color(60), Color::RGB(0, 0, 254));
            }

            #[test]
            fn returns_blue_one_octave_higher() {
                assert_eq!(Areas::make_color(72), Color::RGB(0, 0, 254));
            }

            #[test]
            fn cycles_through_twelve_colors_by_hue_in_cycle_of_fifth() {
                use self::palette::Hsv;
                use self::palette::Srgb;

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

        mod parallelograms {
            use super::*;

            #[test]
            fn renders_a_parallelogram_in_the_bottom_corner() {
                let areas = Areas::parallelograms(800, 600, (6, 10), 6, 36, 7).areas;
                assert_eq!(
                    areas[1].shape,
                    Shape::Parallelogram {
                        base: Position { x: 800, y: 600 },
                        u: Position { x: -6, y: -6 },
                        v: Position { x: 0, y: -10 },
                    }
                );
            }

            #[test]
            fn renders_subsequent_parallelograms_in_the_bottom_row() {
                let areas = Areas::parallelograms(800, 600, (6, 10), 6, 36, 7).areas;
                assert_eq!(
                    areas[2..4]
                        .into_iter()
                        .map(|x| &x.shape)
                        .collect::<Vec<&Shape>>(),
                    vec![
                        &Shape::Parallelogram {
                            base: Position { x: 800, y: 590 },
                            u: Position { x: -6, y: -6 },
                            v: Position { x: 0, y: -10 },
                        },
                        &Shape::Parallelogram {
                            base: Position { x: 800, y: 580 },
                            u: Position { x: -6, y: -6 },
                            v: Position { x: 0, y: -10 },
                        },
                    ]
                );
            }

            #[test]
            fn renders_edge_parallelograms_in_first_row() {
                let areas = Areas::parallelograms(800, 605, (6, 10), 6, 36, 7).areas;
                assert_eq!(
                    areas[61].shape,
                    Shape::Parallelogram {
                        base: Position { x: 800, y: 5 },
                        u: Position { x: -6, y: -6 },
                        v: Position { x: 0, y: -10 },
                    }
                );
            }

            #[test]
            fn first_row_is_chromatic_scale() {
                let areas = Areas::parallelograms(800, 600, (6, 10), 6, 36, 7).areas;
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
                let areas = Areas::parallelograms(800, 600, (6, 10), 6, 36, 7).areas;
                assert_eq!(
                    areas[62..64]
                        .into_iter()
                        .map(|x| &x.shape)
                        .collect::<Vec<&Shape>>(),
                    vec![
                        &Shape::Parallelogram {
                            base: Position { x: 794, y: 594 },
                            u: Position { x: -6, y: -6 },
                            v: Position { x: 0, y: -10 },
                        },
                        &Shape::Parallelogram {
                            base: Position { x: 794, y: 584 },
                            u: Position { x: -6, y: -6 },
                            v: Position { x: 0, y: -10 },
                        },
                    ]
                );
            }

            #[test]
            fn second_row_is_a_fifth_higher() {
                let areas = Areas::parallelograms(800, 600, (6, 10), 6, 36, 7).areas;
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
                let areas = Areas::parallelograms(800, 600, (6, 10), 6, 36, 4).areas;
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
                let areas = Areas::parallelograms(800, 600, (6, 10), 6, 48, 7).areas;
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
                let areas = Areas::parallelograms(800, 600, (6, 10), 3, 36, 4).areas;
                assert_eq!(
                    areas[1].shape,
                    Shape::Parallelogram {
                        base: Position { x: 800, y: 600 },
                        u: Position { x: -6, y: -3 },
                        v: Position { x: 0, y: -10 },
                    }
                );
                assert_eq!(
                    areas[62].shape,
                    Shape::Parallelogram {
                        base: Position { x: 794, y: 597 },
                        u: Position { x: -6, y: -3 },
                        v: Position { x: 0, y: -10 },
                    }
                );
            }

            #[test]
            fn allows_to_configure_parallelogram_width() {
                let areas = Areas::parallelograms(800, 600, (10, 10), 6, 36, 4).areas;
                assert_eq!(
                    areas[1].shape,
                    Shape::Parallelogram {
                        base: Position { x: 800, y: 600 },
                        u: Position { x: -10, y: -6 },
                        v: Position { x: 0, y: -10 },
                    }
                );
                assert_eq!(
                    areas[62].shape,
                    Shape::Parallelogram {
                        base: Position { x: 790, y: 594 },
                        u: Position { x: -10, y: -6 },
                        v: Position { x: 0, y: -10 },
                    }
                );
            }
        }

        mod flipped {
            use super::*;

            #[test]
            fn has_the_base_note_in_the_lower_right_corner() {
                let areas = Areas::flipped(800, 600, 80, 60, 0).areas;
                assert_eq!(
                    areas[0].shape,
                    Shape::Rectangle {
                        x: 0,
                        y: 590,
                        width: 10,
                        height: 10,
                    }
                );
            }

            #[test]
            fn takes_the_screen_size_into_account() {
                let areas = Areas::flipped(8000, 1200, 80, 60, 0).areas;
                assert_eq!(
                    areas[0].shape,
                    Shape::Rectangle {
                        x: 0,
                        y: 1180,
                        width: 100,
                        height: 20,
                    }
                );
            }

            #[test]
            fn renders_the_bottom_row() {
                let areas = Areas::flipped(800, 600, 80, 60, 0).areas;
                for i in 0..80 {
                    assert_eq!(
                        areas[i].shape,
                        Shape::Rectangle {
                            x: 10 * i as i32,
                            y: 590,
                            width: 10,
                            height: 10,
                        },
                        "index: {}",
                        i
                    )
                }
            }

            #[test]
            fn bottom_row_are_semitones() {
                let areas = Areas::flipped(800, 600, 80, 60, 0).areas;
                for i in 0..80 {
                    assert_eq!(areas[i].midi_note, i as i32)
                }
            }

            #[test]
            fn renders_a_second_row() {
                let areas = Areas::flipped(800, 600, 80, 60, 0).areas;
                for i in 0..80 {
                    assert_eq!(
                        areas[i as usize + 80],
                        Area::new(
                            Shape::Rectangle {
                                x: 10 * i,
                                y: 580,
                                width: 10,
                                height: 10,
                            },
                            i + 5
                        )
                    )
                }
            }

            #[test]
            fn renders_the_top_row() {
                let areas = Areas::flipped(800, 600, 80, 60, 0).areas;
                for i in 0..80 {
                    assert_eq!(
                        areas[i as usize + 80 * 59],
                        Area::new(
                            Shape::Rectangle {
                                x: 10 * i,
                                y: 0,
                                width: 10,
                                height: 10,
                            },
                            i + 5 * 59
                        )
                    );
                }
            }

            #[test]
            fn allows_to_configure_the_number_of_rectangles() {
                assert_eq!(Areas::flipped(800, 600, 80, 60, 0).areas.len(), 80 * 60);
                assert_eq!(Areas::flipped(800, 600, 10, 6, 0).areas.len(), 10 * 6);
            }

            #[test]
            fn allows_to_configure_the_base_note() {
                let areas = Areas::flipped(800, 600, 80, 60, 36).areas;
                assert_eq!(areas[0].midi_note, 36);
                assert_eq!(areas[1].midi_note, 37);
                assert_eq!(areas[80].midi_note, 41);
            }
        }
    }
}
