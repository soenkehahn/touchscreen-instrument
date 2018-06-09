extern crate sdl2;

use evdev::Position;

#[derive(Debug, Clone)]
pub struct Rectangle {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub midi_note: i32,
}

impl Rectangle {
    pub fn contains(&self, position: Position) -> bool {
        let x_in = position.x >= self.x && position.x < self.x + self.width;
        let y_in = position.y >= self.y && position.y < self.y + self.height;
        x_in && y_in
    }

    pub fn midi_note(&self) -> i32 {
        self.midi_note
    }

    pub fn to_sdl_rect(&self, x_factor: f32, y_factor: f32) -> sdl2::rect::Rect {
        sdl2::rect::Rect::new(
            (self.x as f32 * x_factor) as i32,
            (self.y as f32 * y_factor) as i32,
            (self.width as f32 * x_factor) as u32,
            (self.height as f32 * y_factor) as u32,
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;

    mod contains {
        use super::*;

        const RECTANGLE: Rectangle = Rectangle {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
            midi_note: 60,
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
}
