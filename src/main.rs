#![feature(type_ascription)]

extern crate futures;
extern crate futures_fs;

use futures_fs::FsPool;
use futures::stream::Stream;

fn main() {
    let fs = FsPool::default();
    let read = fs.read("/dev/input/mouse0", Default::default());
    for foo in read.wait() {
        println!("hooray: {:?}", foo);
    }
}
