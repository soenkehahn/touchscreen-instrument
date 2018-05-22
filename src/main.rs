#![feature(type_ascription)]

extern crate bytes;
extern crate futures;
extern crate futures_fs;

use futures_fs::FsPool;
use futures::stream::Stream;
use bytes::Bytes;
use std::io::*;
use std::mem::*;

fn main() {
    let fs = FsPool::default();
    let read = fs.read("/dev/input/mouse0", Default::default());
    for foo in read.wait() {
        let bar: Result<Bytes> = foo;
        match bar {
            Ok(v) => {
                if v.len() != 3 {
                    panic!("expected: length of 3");
                }
                let (x, y): (i8, i8) = unsafe { (transmute(v[1]), transmute(v[2])) };
                print!("\rx: {:4}, y: {:4}", x, y);
                let _ = stdout().flush();
            }
            Err(e) => panic!(e),
        }
    }
}
