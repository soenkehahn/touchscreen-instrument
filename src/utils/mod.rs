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

#[cfg(test)]
pub fn mk_slots<T, F: FnMut(usize) -> T>(mut f: F) -> Slots<T> {
    let mut indices = [0; 10];
    for (i, slot) in indices.iter_mut().enumerate() {
        *slot = i;
    }
    slot_map(indices, |i| f(*i))
}

#[cfg(test)]
pub fn slot_map<F, T, U>(input: Slots<T>, mut f: F) -> Slots<U>
where
    F: FnMut(&T) -> U,
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
