use areas::{Areas, Orientation, ParallelogramConfig};
use evdev::Position;

pub fn parallelograms(touch_width: i32, touch_height: i32) -> Areas {
    Areas::new(ParallelogramConfig {
        touch_width,
        touch_height,
        orientation: Orientation::Portrait,
        u: Position { x: 0, y: -1300 },
        v: Position { x: -1000, y: -200 },
        column_range: (-3, 8),
        row_range: (0, 17),
        start_midi_note: 24,
        row_interval: 5,
    })
}

pub fn grid(
    touch_width: i32,
    touch_height: i32,
    row_length: i32,
    number_of_rows: i32,
    start_midi_note: i32,
) -> Areas {
    Areas::new(ParallelogramConfig {
        touch_width,
        touch_height,
        orientation: Orientation::Landscape,
        u: Position {
            x: touch_width / row_length,
            y: 0,
        },
        v: Position {
            x: 0,
            y: -touch_height / number_of_rows,
        },
        column_range: (0, row_length),
        row_range: (0, number_of_rows),
        start_midi_note,
        row_interval: 5,
    })
}

#[cfg(test)]
mod test {
    use super::*;
    mod grid {
        use super::*;
        use areas::shape::Shape;
        use areas::Area;

        #[test]
        fn has_the_base_note_in_the_lower_right_corner() {
            let areas = grid(800, 600, 80, 60, 0).areas;
            assert_eq!(
                areas[0].shape,
                Shape::Parallelogram {
                    base: Position { x: 0, y: 600 },
                    u: Position { x: 10, y: 0 },
                    v: Position { x: 0, y: -10 },
                }
            );
        }

        #[test]
        fn takes_the_screen_size_into_account() {
            let areas = grid(8000, 1200, 80, 60, 0).areas;
            assert_eq!(
                areas[0].shape,
                Shape::Parallelogram {
                    base: Position { x: 0, y: 1200 },
                    u: Position { x: 100, y: 0 },
                    v: Position { x: 0, y: -20 },
                }
            );
        }

        #[test]
        fn renders_the_bottom_row() {
            let areas = grid(800, 600, 80, 60, 0).areas;
            for i in 0..80 {
                assert_eq!(
                    areas[i].shape,
                    Shape::Parallelogram {
                        base: Position {
                            x: 10 * i as i32,
                            y: 600
                        },
                        u: Position { x: 10, y: 0 },
                        v: Position { x: 0, y: -10 },
                    },
                    "index: {}",
                    i
                )
            }
        }

        #[test]
        fn bottom_row_are_semitones() {
            let areas = grid(800, 600, 80, 60, 0).areas;
            for i in 0..80 {
                assert_eq!(areas[i].midi_note, i as i32)
            }
        }

        #[test]
        fn renders_a_second_row() {
            let areas = grid(800, 600, 80, 60, 0).areas;
            for i in 0..80 {
                assert_eq!(
                    areas[i as usize + 80],
                    Area::new(
                        Shape::Parallelogram {
                            base: Position { x: 10 * i, y: 590 },
                            u: Position { x: 10, y: 0 },
                            v: Position { x: 0, y: -10 },
                        },
                        i + 5
                    )
                )
            }
        }

        #[test]
        fn renders_the_top_row() {
            let areas = grid(800, 600, 80, 60, 0).areas;
            for i in 0..80 {
                assert_eq!(
                    areas[i as usize + 80 * 59],
                    Area::new(
                        Shape::Parallelogram {
                            base: Position { x: 10 * i, y: 10 },
                            u: Position { x: 10, y: 0 },
                            v: Position { x: 0, y: -10 },
                        },
                        i + 5 * 59
                    )
                );
            }
        }

        #[test]
        fn allows_to_configure_the_number_of_rectangles() {
            assert_eq!(grid(800, 600, 80, 60, 0).areas.len(), 80 * 60);
            assert_eq!(grid(800, 600, 10, 6, 0).areas.len(), 10 * 6);
        }

        #[test]
        fn allows_to_configure_the_base_note() {
            let areas = grid(800, 600, 80, 60, 36).areas;
            assert_eq!(areas[0].midi_note, 36);
            assert_eq!(areas[1].midi_note, 37);
            assert_eq!(areas[80].midi_note, 41);
        }
    }
}
