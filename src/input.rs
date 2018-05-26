use std::io::Read;
use std::iter::Iterator;
use std::*;

// * chunks

struct Chunks<R: Read> {
    read: Box<R>,
}

impl<R: Read> Chunks<R> {
    fn new(read: R) -> Chunks<R> {
        Chunks {
            read: Box::new(read),
        }
    }
}

impl<'a, R: Read> Iterator for Chunks<R> {
    type Item = [u8; 3];

    fn next(&mut self) -> Option<[u8; 3]> {
        let mut buffer = [0; 3];
        match self.read.read(&mut buffer) {
            Ok(_len) => {
                return Some(buffer);
            }
            Err(e) => {
                panic!(e);
            }
        };
    }
}

// * diffs

#[derive(PartialEq, Debug)]
struct Diffs {
    x: i8,
    y: i8,
}

fn parse(chunk: [u8; 3]) -> Diffs {
    unsafe {
        Diffs {
            x: mem::transmute::<u8, i8>(chunk[1]),
            y: mem::transmute::<u8, i8>(chunk[2]),
        }
    }
}

// * Position

#[derive(PartialEq, Debug, Clone, Copy)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

// * mouse input

pub struct MouseInput<R: Read> {
    first: bool,
    chunks: Box<Chunks<R>>,
    position: Position,
}

impl<R: Read> MouseInput<R> {
    pub fn new(read: R) -> MouseInput<R> {
        MouseInput {
            first: true,
            chunks: Box::new(Chunks::new(read)),
            position: Position { x: 0, y: 0 },
        }
    }
}

impl<R: Read> Iterator for MouseInput<R> {
    type Item = Position;

    fn next(&mut self) -> Option<Position> {
        if self.first {
            self.first = false;
            return Some(self.position);
        }
        match self.chunks.next() {
            None => None,
            Some(chunk) => {
                let diff = parse(chunk);
                self.position.x += diff.x as i32;
                self.position.y += diff.y as i32;
                Some(self.position)
            }
        }
    }
}

test_suite! {
    use super::*;

    struct ReadMock {
        current: usize,
        vec: Vec<Vec<u8>>,
    }
    impl ReadMock {
        fn new(vec: Vec<Vec<u8>>) -> ReadMock {
            ReadMock { current: 0, vec }
        }
    }
    impl Read for ReadMock {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            match self.vec.iter().nth(self.current){
                None => {
                    panic!("empty read mock");
                }
                Some(chunk) => {
                    for (i, e) in chunk.iter().enumerate() {
                        buffer[i] = *e;
                    }
                    self.current += 1;
                    Ok(self.vec.len())
                }
            }
        }
    }

    test chunks_converts_single_chunk() {
        let mut read_mock = ReadMock::new(vec![vec![1, 2, 3]]);
        let mut iterator = Chunks::new(&mut read_mock);
        assert_eq!(iterator.next(), Some([1, 2, 3]));
    }

    test chunks_converts_multiple_chunks() {
        let mut read_mock = ReadMock::new(vec![vec![1, 2, 3], vec![4, 5, 6]]);
        let mut iterator = Chunks::new(&mut read_mock);
        iterator.next();
        assert_eq!(iterator.next(), Some([4, 5, 6]));
    }

    test parse_parses_chunks() {
        assert_eq!(parse([0, 1, 2]), Diffs {x: 1, y: 2});
    }

    test parse_parses_as_signed_integers() {
        assert_eq!(parse([0, 255, 254]), Diffs {x: -1, y: -2});
    }

    test mouse_input_starts_with_an_initial_position() {
        let read_mock = &mut ReadMock::new(vec![]);
        let mut input = MouseInput::new(read_mock);
        assert_eq!(input.next(), Some(Position{x: 0, y: 0}));
    }

    test mouse_input_maintains_position_state() {
        let read_mock = &mut ReadMock::new(vec![vec![0, 1, 2], vec![0, 3, 4]]);
        let mut input = MouseInput::new(read_mock);
        input.next();
        assert_eq!(input.next(), Some(Position{x: 1, y: 2}));
        assert_eq!(input.next(), Some(Position{x: 4, y: 6}));
    }
}
