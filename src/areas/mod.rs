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

#[derive(Clone)]
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
    touch_width: u32,
    touch_height: u32,
}

impl Areas {
    #[allow(dead_code)]
    pub fn stripes(
        touch_width: u32,
        touch_height: u32,
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

    #[allow(dead_code)]
    pub fn peas(touch_width: u32, touch_height: u32, rect_size: i32) -> Areas {
        let mut areas = vec![];
        for row in 0..4 {
            for i in 0..36 {
                let row_offset = -((2.5 * rect_size as f32 * row as f32) as i32 + 2 * rect_size);
                let note_is_even = i % 2 == 0;
                let pea_offset = if note_is_even { rect_size } else { 0 };
                areas.push(Area::new(
                    Shape::Rectangle {
                        x: i * rect_size / 2,
                        y: touch_height as i32 + pea_offset + row_offset,
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

    pub fn frequency(&self, position: Position) -> NoteEvent {
        let touched: Option<&Area> = self.areas
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
    }
}
