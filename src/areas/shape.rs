extern crate sdl2;

use evdev::Position;

#[derive(Debug, Clone, PartialEq)]
pub enum Shape {
    Rectangle {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
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
        }
    }

    pub fn to_polygon(&self, x_factor: f32, y_factor: f32) -> (Box<[i16]>, Box<[i16]>) {
        match self {
            Shape::Rectangle {
                x,
                y,
                width,
                height,
                ..
            } => {
                let x1 = (*x as f32 * x_factor) as i16;
                let y1 = (*y as f32 * y_factor) as i16;
                let x2 = x1 + (*width as f32 * x_factor) as i16;
                let y2 = y1 + (*height as f32 * y_factor) as i16;
                (Box::new([x1, x2, x2, x1]), Box::new([y1, y1, y2, y2]))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod contains {
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

    mod to_polygon {
        use areas::Areas;

        #[test]
        fn translates_touch_coordinates_to_screen_coordinates() {
            let areas = Areas::stripes(1000, 1000, 10, 48).areas;
            let expected: (Box<[i16]>, Box<[i16]>) =
                (Box::new([14, 21, 21, 14]), Box::new([0, 0, 5000, 5000]));
            assert_eq!(
                areas
                    .get(2)
                    .unwrap()
                    .shape
                    .to_polygon(700.0 / 1000.0, 500.0 / 1000.0),
                expected
            );
        }

        #[test]
        fn factors_in_the_area_size() {
            let areas = Areas::stripes(1000, 1000, 12, 48).areas;
            let x1 = (24.0 * 0.7) as i16;
            let x2 = x1 + (12.0 * 0.7) as i16;
            let y1 = (1.0 * 0.5) as i16;
            let y2 = y1 + (10000.0 * 0.5) as i16;
            let expected: (Box<[i16]>, Box<[i16]>) =
                (Box::new([x1, x2, x2, x1]), Box::new([y1, y1, y2, y2]));
            assert_eq!(
                areas
                    .get(2)
                    .unwrap()
                    .shape
                    .to_polygon(700.0 / 1000.0, 500.0 / 1000.0),
                expected
            );
        }
    }
}
