pub mod thread_worker;

use std::marker::PhantomData;
use std::time::Duration;

pub fn blocking<T>() -> Blocking<T> {
    Blocking(PhantomData)
}

pub struct Blocking<T>(PhantomData<T>);

impl<T> Iterator for Blocking<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        loop {
            ::std::thread::sleep(Duration::new(1, 0));
        }
    }
}

pub type Slots<T> = [T; 10];

pub fn new_slots<F, T>(f: F) -> Slots<T>
where
    F: Fn() -> T,
{
    slot_map([(); 10], |()| f())
}

pub fn slot_map<F, T, U>(input: Slots<T>, f: F) -> Slots<U>
where
    F: Fn(&T) -> U,
{
    [
        f(&input[0]),
        f(&input[1]),
        f(&input[2]),
        f(&input[3]),
        f(&input[4]),
        f(&input[5]),
        f(&input[6]),
        f(&input[7]),
        f(&input[8]),
        f(&input[9]),
    ]
}
