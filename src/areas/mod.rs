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

#[derive(Clone, Debug)]
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

#[derive(Clone)]
pub struct Areas {
    areas: Vec<Area>,
    touch_width: i32,
    touch_height: i32,
}

impl Areas {
    pub fn stripes(
        touch_width: i32,
        touch_height: i32,
        rect_size: i32,
        start_midi_note: i32,
    ) -> Areas {
        let mut areas = vec![];
        for i in 0..30 {
            areas.push(Area::new(
                Shape::Rectangle {
                    x: i * rect_size,
                    y: 1,
                    width: rect_size,
                    height: 10000,
                },
                start_midi_note + i,
            ));
        }
        Areas {
            areas,
            touch_width,
            touch_height,
        }
    }

    pub fn peas(touch_width: i32, touch_height: i32, rect_size: i32) -> Areas {
        let mut areas = vec![];
        for row in 0..4 {
            for i in 0..36 {
                let row_offset = -((2.5 * rect_size as f32 * row as f32) as i32 + 2 * rect_size);
                let note_is_even = i % 2 == 0;
                let pea_offset = if note_is_even { rect_size } else { 0 };
                areas.push(Area::new(
                    Shape::Rectangle {
                        x: i * rect_size / 2,
                        y: touch_height + pea_offset + row_offset,
                        width: rect_size,
                        height: rect_size,
                    },
                    36 + i + row * 12,
                ));
            }
        }
        Areas {
            areas,
            touch_width,
            touch_height,
        }
    }

    pub fn triangles(touch_width: i32, touch_height: i32, size: i32) -> Areas {
        let mut areas = vec![];
        let half = size / 2;
        for row in 0..8 {
            for i in 0..30 {
                let i_even = i % 2 == 0;
                let row_offset = touch_height - size * row;
                let row_even = row % 2 == 0;
                let x_offset = if row_even { 0 } else { -half };
                let triangle = if i_even {
                    Shape::Triangle {
                        a: Position {
                            x: x_offset + touch_width - i * half,
                            y: row_offset,
                        },
                        b: Position {
                            x: x_offset + touch_width - (i * half + half),
                            y: row_offset - size,
                        },
                        c: Position {
                            x: x_offset + touch_width - (i * half + size),
                            y: row_offset,
                        },
                    }
                } else {
                    Shape::Triangle {
                        a: Position {
                            x: x_offset + touch_width - ((i - 1) * half + half),
                            y: row_offset - size,
                        },
                        b: Position {
                            x: x_offset + touch_width - ((i - 1) * half + size),
                            y: row_offset,
                        },
                        c: Position {
                            x: x_offset + touch_width - ((i - 1) * half + size + half),
                            y: row_offset - size,
                        },
                    }
                };
                let midi_offset = if row % 2 == 0 {
                    row / 2 * 12
                } else {
                    (row - 1) / 2 * 12 + 7
                };
                areas.push(Area::new(triangle, 36 + midi_offset + i));
            }
        }
        Areas {
            areas,
            touch_width,
            touch_height,
        }
    }

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
    use sound::NoteEvent::*;

    fn pos(x: i32) -> Position {
        Position { x, y: 5 }
    }

    mod areas {
        use super::*;

        mod frequency {
            use super::*;

            mod stripes {
                use super::*;

                #[test]
                fn maps_x_values_to_frequencies() {
                    let areas = Areas::stripes(800, 600, 10, 48);
                    assert_eq!(areas.frequency(pos(5)), NoteOn(midi_to_frequency(48)));
                }

                #[test]
                fn maps_higher_x_values_to_higher_frequencies() {
                    let areas = Areas::stripes(800, 600, 10, 48);
                    assert_eq!(areas.frequency(pos(15)), NoteOn(midi_to_frequency(49)));
                }

                #[test]
                fn has_non_continuous_steps() {
                    let areas = Areas::stripes(800, 600, 10, 48);
                    assert_eq!(areas.frequency(pos(9)), NoteOn(midi_to_frequency(48)));
                    assert_eq!(areas.frequency(pos(10)), NoteOn(midi_to_frequency(49)));
                }

                #[test]
                fn allows_to_change_area_size() {
                    let areas = Areas::stripes(800, 600, 12, 48);
                    assert_eq!(areas.frequency(pos(11)), NoteOn(midi_to_frequency(48)));
                    assert_eq!(areas.frequency(pos(12)), NoteOn(midi_to_frequency(49)));
                }
            }
        }

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

                let mut color: Hsv = Srgb::new(0.0, 0.0, 1.0).into();
                assert_eq!(
                    Areas::make_color(60),
                    Areas::convert_color(Srgb::from(color).into_format())
                );
                color.hue = color.hue + 360.0 / 12.0;
                assert_eq!(
                    Areas::make_color(67),
                    Areas::convert_color(Srgb::from(color).into_format())
                );
                color.hue = color.hue + 360.0 / 12.0;
                assert_eq!(
                    Areas::make_color(62),
                    Areas::convert_color(Srgb::from(color).into_format())
                );
            }
        }

        mod stripes {
            use super::*;

            #[test]
            fn returns_a_rectangle_for_the_lowest_pitch() {
                let areas = Areas::stripes(800, 600, 10, 48).areas;
                assert_eq!(
                    areas.get(0).unwrap().shape,
                    Shape::Rectangle {
                        x: 0,
                        y: 1,
                        width: 10,
                        height: 10000
                    }
                );
            }

            #[test]
            fn returns_rectangles_for_higher_pitches() {
                let areas = Areas::stripes(800, 600, 10, 48).areas;
                assert_eq!(
                    areas.get(1).unwrap().shape,
                    Shape::Rectangle {
                        x: 10,
                        y: 1,
                        width: 10,
                        height: 10000
                    }
                );
                assert_eq!(
                    areas.get(2).unwrap().shape,
                    Shape::Rectangle {
                        x: 20,
                        y: 1,
                        width: 10,
                        height: 10000
                    }
                );
            }

            #[test]
            fn returns_blue_for_c() {
                let areas = Areas::stripes(800, 600, 10, 60).areas;
                assert_eq!(areas.get(0).unwrap().color, Color::RGB(0, 0, 254));
            }

            #[test]
            fn returns_blue_for_c_when_starting_at_different_notes() {
                let areas = Areas::stripes(800, 600, 10, 59).areas;
                assert_eq!(areas.get(1).unwrap().color, Color::RGB(0, 0, 254));
            }
        }

        mod peas {
            use super::*;

            #[test]
            fn returns_correct_rectangles_in_the_lowest_row() {
                let areas = Areas::peas(800, 600, 10);
                let areas = areas.areas;
                assert_eq!(
                    areas[0].shape,
                    Shape::Rectangle {
                        x: 0,
                        y: 590,
                        width: 10,
                        height: 10,
                    }
                );
                assert_eq!(
                    areas[1].shape,
                    Shape::Rectangle {
                        x: 5,
                        y: 580,
                        width: 10,
                        height: 10,
                    }
                );
                assert_eq!(
                    areas[2].shape,
                    Shape::Rectangle {
                        x: 10,
                        y: 590,
                        width: 10,
                        height: 10
                    }
                );
            }

            #[test]
            fn returns_multiple_rows() {
                let areas = Areas::peas(800, 600, 10).areas;
                assert_eq!(
                    areas[36].shape,
                    Shape::Rectangle {
                        x: 0,
                        y: 565,
                        width: 10,
                        height: 10
                    }
                );
                assert_eq!(
                    areas[37].shape,
                    Shape::Rectangle {
                        x: 5,
                        y: 555,
                        width: 10,
                        height: 10
                    }
                );
                assert_eq!(
                    areas[38].shape,
                    Shape::Rectangle {
                        x: 10,
                        y: 565,
                        width: 10,
                        height: 10
                    }
                );
            }

            #[test]
            fn subsequent_rows_are_one_octaves_higher() {
                let areas = Areas::peas(800, 600, 10).areas;
                assert_eq!(areas[0].midi_note, 36);
                assert_eq!(areas[36].midi_note, 36 + 12);
            }
        }

        mod triangles {
            use super::*;

            #[test]
            fn includes_the_first_row() {
                let areas = Areas::triangles(800, 600, 10).areas;
                assert_eq!(
                    areas[0].shape,
                    Shape::Triangle {
                        a: Position { x: 800, y: 600 },
                        b: Position { x: 795, y: 590 },
                        c: Position { x: 790, y: 600 },
                    },
                );
                assert_eq!(
                    areas[1].shape,
                    Shape::Triangle {
                        a: Position { x: 795, y: 590 },
                        b: Position { x: 790, y: 600 },
                        c: Position { x: 785, y: 590 },
                    }
                );
                assert_eq!(
                    areas[2].shape,
                    Shape::Triangle {
                        a: Position { x: 790, y: 600 },
                        b: Position { x: 785, y: 590 },
                        c: Position { x: 780, y: 600 },
                    },
                );
                assert_eq!(
                    areas[3].shape,
                    Shape::Triangle {
                        a: Position { x: 785, y: 590 },
                        b: Position { x: 780, y: 600 },
                        c: Position { x: 775, y: 590 },
                    }
                );
            }

            #[test]
            fn has_a_chromatic_scale() {
                let areas = Areas::triangles(800, 600, 10).areas;
                let vec: Vec<i32> = areas
                    .into_iter()
                    .map(|area| area.midi_note)
                    .take(5)
                    .collect();
                assert_eq!(vec, vec![36, 37, 38, 39, 40]);
            }

            #[test]
            fn has_multiple_rows() {
                let areas = Areas::triangles(800, 600, 10).areas;
                assert_eq!(
                    areas[60].shape,
                    Shape::Triangle {
                        a: Position { x: 800, y: 580 },
                        b: Position { x: 795, y: 570 },
                        c: Position { x: 790, y: 580 },
                    }
                );
                assert_eq!(
                    areas[63].shape,
                    Shape::Triangle {
                        a: Position { x: 785, y: 570 },
                        b: Position { x: 780, y: 580 },
                        c: Position { x: 775, y: 570 },
                    }
                );
            }

            #[test]
            fn has_intermediate_rows() {
                let areas = Areas::triangles(800, 600, 10).areas;
                assert_eq!(
                    areas[30].shape,
                    Shape::Triangle {
                        a: Position { x: 795, y: 590 },
                        b: Position { x: 790, y: 580 },
                        c: Position { x: 785, y: 590 },
                    },
                );
                assert_eq!(
                    areas[33].shape,
                    Shape::Triangle {
                        a: Position { x: 780, y: 580 },
                        b: Position { x: 775, y: 590 },
                        c: Position { x: 770, y: 580 },
                    }
                );
            }

            #[test]
            fn rows_are_one_octave_apart() {
                let areas = Areas::triangles(800, 600, 10).areas;
                let vec: Vec<i32> = areas
                    .into_iter()
                    .map(|area| area.midi_note)
                    .skip(60)
                    .take(3)
                    .collect();
                assert_eq!(vec, vec![48, 49, 50]);
            }

            #[test]
            fn intermediat_rows_are_a_fifth_apart() {
                let areas = Areas::triangles(800, 600, 10).areas;
                let vec: Vec<i32> = areas
                    .into_iter()
                    .map(|area| area.midi_note)
                    .skip(30)
                    .take(3)
                    .collect();
                assert_eq!(vec, vec![43, 44, 45]);
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
    }
}
