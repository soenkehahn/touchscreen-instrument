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

pub fn mk_slots<T: Clone>(element: T) -> Slots<T> {
    [
        element.clone(),
        element.clone(),
        element.clone(),
        element.clone(),
        element.clone(),
        element.clone(),
        element.clone(),
        element.clone(),
        element.clone(),
        element,
    ]
}
