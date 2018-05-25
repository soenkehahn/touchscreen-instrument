use std::io::Read;
use std::iter::Iterator;
use std::*;

fn chunks<'a, R: Read>(read: &'a mut R) -> Chunks<'a> {
    Chunks { read }
}

struct Chunks<'a> {
    read: &'a mut Read,
}

impl<'a> Iterator for Chunks<'a> {
    // fixme: dynamic size?
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

#[derive(PartialEq, Debug)]
struct Diffs {
    x_diff: i8,
    y_diff: i8,
}

fn parse(chunk: [u8; 3]) -> Diffs {
    unsafe {
        Diffs {
            x_diff: mem::transmute::<u8, i8>(chunk[1]),
            y_diff: mem::transmute::<u8, i8>(chunk[2]),
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
            for (i, e) in self.vec.iter().nth(self.current).unwrap().iter().enumerate() {
                buffer[i] = *e;
            }
            self.current += 1;
            Ok(self.vec.len())
        }
    }

    test chunks_converts_single_chunk() {
        let mut read_mock = ReadMock::new(vec![vec![1, 2, 3]]);
        let mut iterator = chunks(&mut read_mock);
        assert_eq!(iterator.next(), Some([1, 2, 3]));
    }

    test chunks_converts_multiple_chunks() {
        let mut read_mock = ReadMock::new(vec![vec![1, 2, 3], vec![4, 5, 6]]);
        let mut iterator = chunks(&mut read_mock);
        iterator.next();
        assert_eq!(iterator.next(), Some([4, 5, 6]));
    }

    test parse_parses_chunks() {
        assert_eq!(parse([0, 1, 2]), Diffs {x_diff: 1, y_diff: 2});
    }

    test parse_parses_as_signed_integers() {
        assert_eq!(parse([0, 255, 254]), Diffs {x_diff: -1, y_diff: -2});
    }
}
