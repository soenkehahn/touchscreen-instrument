#![feature(type_ascription)]

extern crate bytes;
extern crate futures;
extern crate futures_fs;

use bytes::Bytes;
use futures::stream::Stream;
use futures_fs::FsPool;
use std::io::stdout;
use std::io::Error;
use std::io::Write;
use std::mem::*;
use std::result::*;

fn print_position(x: i32, y: i32) {
    print!("\r\x1b[Kx: {:4}, y: {:4}", x, y);
    let _ = stdout().flush();
}

fn main_() -> Result<(), Error> {
    let fs = FsPool::default();
    let read = fs.read("/dev/input/mouse0", Default::default());
    let mut x: i32 = 0;
    let mut y: i32 = 0;
    print_position(x, y);
    for foo in read.wait() {
        let v: Bytes = foo?;
        if v.len() != 3 {
            panic!("expected: length of 3");
        }
        let (x_diff, y_diff): (i8, i8) = unsafe { (transmute(v[1]), transmute(v[2])) };
        x += x_diff as i32;
        y += y_diff as i32;
        print_position(x, y);
    }
    Ok(())
}

fn main() {
    match main_() {
        Ok(()) => {}
        Err(e) => {
            panic!(e);
        }
    }
}
