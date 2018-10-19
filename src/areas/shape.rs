extern crate sdl2;

use evdev::Position;

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    #[allow(dead_code)]
    Rectangle {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    Parallelogram {
        base: Position,
        u: Position,
        v: Position,
    },
}

impl Shape {
    pub fn contains(&self, position: Position) -> bool {
        match *self {
            Shape::Rectangle {
                x,
                y,
                width,
                height,
                ..
            } => {
                let x_in = position.x >= x && position.x < x + width;
                let y_in = position.y >= y && position.y < y + height;
                x_in && y_in
            }
            Shape::Parallelogram { base, u, v } => {
                let translated_position = Position {
                    x: position.x - base.x,
                    y: position.y - base.y,
                };
                let multiplication_factor = 1.0 / (u.x * v.y - v.x * u.y) as f32;
                let u_component = (translated_position.x * v.y + translated_position.y * (-v.x))
                    as f32
                    * multiplication_factor;
                let v_component = (translated_position.x * (-u.y) + translated_position.y * u.x)
                    as f32
                    * multiplication_factor;
                u_component >= 0.0 && u_component <= 1.0 && v_component >= 0.0 && v_component <= 1.0
            }
        }
    }

    pub fn to_polygon(&self, x_factor: f32, y_factor: f32) -> (Box<[i16]>, Box<[i16]>) {
        let (mut xs, mut ys): (Box<[i16]>, Box<[i16]>) = match self {
            Shape::Rectangle {
                x,
                y,
                width,
                height,
                ..
            } => {
                let x1 = *x as i16;
                let y1 = *y as i16;
                let x2 = x1 + *width as i16;
                let y2 = y1 + *height as i16;
                (Box::new([x1, x2, x2, x1]), Box::new([y1, y1, y2, y2]))
            }
            Shape::Parallelogram { base, u, v } => (
                Box::new([
                    base.x as i16,
                    (u.x + base.x) as i16,
                    (u.x + v.x + base.x) as i16,
                    (v.x + base.x) as i16,
                ]),
                Box::new([
                    base.y as i16,
                    (u.y + base.y) as i16,
                    (u.y + v.y + base.y) as i16,
                    (v.y + base.y) as i16,
                ]),
            ),
        };
        for x in xs.iter_mut() {
            *x = (*x as f32 * x_factor) as i16;
        }
        for y in ys.iter_mut() {
            *y = (*y as f32 * y_factor) as i16;
        }
        (xs, ys)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod contains {
        use super::*;

        mod rectangle {
            use super::*;

            const RECTANGLE: Shape = Shape::Rectangle {
                x: 0,
                y: 0,
                width: 10,
                height: 10,
            };

            #[test]
            fn detects_positions_inside() {
                assert!(RECTANGLE.contains(Position { x: 5, y: 5 }))
            }

            #[test]
            fn returns_false_for_positions_to_the_left() {
                assert!(!RECTANGLE.contains(Position { x: -5, y: 5 }))
            }

            #[test]
            fn returns_false_for_positions_to_the_right() {
                assert!(!RECTANGLE.contains(Position { x: 15, y: 5 }))
            }

            #[test]
            fn returns_false_for_positions_above() {
                assert!(!RECTANGLE.contains(Position { x: 5, y: 15 }))
            }

            #[test]
            fn returns_false_for_positions_below() {
                assert!(!RECTANGLE.contains(Position { x: 5, y: -5 }))
            }
        }

        mod parallelogram {
            use super::*;

            const PARALLELOGRAM: Shape = Shape::Parallelogram {
                base: Position { x: 0, y: 0 },
                u: Position { x: 10, y: 5 },
                v: Position { x: 5, y: 10 },
            };

            #[test]
            fn detects_positions_inside() {
                assert!(PARALLELOGRAM.contains(Position { x: 5, y: 5 }));
                assert!(PARALLELOGRAM.contains(Position { x: 10, y: 10 }));
                assert!(PARALLELOGRAM.contains(Position { x: 5, y: 9 }));
            }

            #[test]
            fn returns_true_for_corners() {
                assert!(PARALLELOGRAM.contains(Position { x: 0, y: 0 }));
                assert!(PARALLELOGRAM.contains(Position { x: 5, y: 10 }));
                assert!(PARALLELOGRAM.contains(Position { x: 15, y: 15 }));
                assert!(PARALLELOGRAM.contains(Position { x: 10, y: 5 }));
            }

            #[test]
            fn detects_positions_outside() {
                assert!(!PARALLELOGRAM.contains(Position { x: 2, y: 8 }));
                assert!(!PARALLELOGRAM.contains(Position { x: 7, y: 13 }));
                assert!(!PARALLELOGRAM.contains(Position { x: 13, y: 7 }));
                assert!(!PARALLELOGRAM.contains(Position { x: 8, y: 2 }));
            }

            #[test]
            fn works_for_translated_parallelograms() {
                let parallelogram = Shape::Parallelogram {
                    base: Position { x: 10, y: 10 },
                    u: Position { x: 10, y: 0 },
                    v: Position { x: 0, y: 10 },
                };
                assert!(!parallelogram.contains(Position { x: 5, y: 5 }));
                assert!(!parallelogram.contains(Position { x: 15, y: 5 }));
                assert!(!parallelogram.contains(Position { x: 5, y: 15 }));
                assert!(parallelogram.contains(Position { x: 15, y: 15 }));
            }
        }
    }

    mod to_polygon {
        use super::*;

        mod parallelogram {
            use super::*;

            #[test]
            fn converts_correctly_base_zero() {
                let parallelogram = Shape::Parallelogram {
                    base: Position { x: 0, y: 0 },
                    u: Position { x: 10, y: 5 },
                    v: Position { x: 5, y: 10 },
                };

                let expected: (Box<[i16]>, Box<[i16]>) =
                    (Box::new([0, 10, 15, 5]), Box::new([0, 5, 15, 10]));
                assert_eq!(parallelogram.to_polygon(1.0, 1.0), expected);
            }

            #[test]
            fn converts_correctly_base_nonzero() {
                let parallelogram = Shape::Parallelogram {
                    base: Position { x: 33, y: 77 },
                    u: Position { x: 10, y: 5 },
                    v: Position { x: 5, y: 10 },
                };

                let expected: (Box<[i16]>, Box<[i16]>) =
                    (Box::new([33, 43, 48, 38]), Box::new([77, 82, 92, 87]));
                assert_eq!(parallelogram.to_polygon(1.0, 1.0), expected);
            }

            #[test]
            fn converts_touch_coordinates_correctly() {
                let parallelogram = Shape::Parallelogram {
                    base: Position { x: 0, y: 0 },
                    u: Position { x: 1, y: 0 },
                    v: Position { x: 0, y: 1 },
                };
                let expected: (Box<[i16]>, Box<[i16]>) =
                    (Box::new([0, 2, 2, 0]), Box::new([0, 0, 3, 3]));
                assert_eq!(parallelogram.to_polygon(2.0, 3.0), expected);
            }
        }

        mod parallelograms {
            use areas::Areas;

            #[test]
            fn translates_touch_coordinates_to_screen_coordinates() {
                let screen_polygon = Areas::parallelograms(1000, 1000, (10, 10), 0, 48, 7)
                    .areas
                    .get(1)
                    .unwrap()
                    .shape
                    .to_polygon(700.0 / 1000.0, 500.0 / 1000.0);
                let expected: (Box<[i16]>, Box<[i16]>) = (
                    Box::new([700, 693, 693, 700]),
                    Box::new([500, 500, 495, 495]),
                );
                assert_eq!(screen_polygon, expected);
            }
        }
    }
}
