extern crate chrono;

use self::chrono::prelude::*;
use jack::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct XRunLogger {
    counter: Arc<AtomicUsize>,
}

impl XRunLogger {
    pub fn new_and_spawn() -> XRunLogger {
        let result = XRunLogger::new();
        let mut clone = result.clone();
        ::std::thread::spawn(move || loop {
            ::std::thread::sleep(::std::time::Duration::new(1, 0));
            clone.print_output();
        });
        result
    }

    fn new() -> XRunLogger {
        XRunLogger {
            counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn xrun_(&mut self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
    }

    fn output(&mut self) -> Option<String> {
        let counter = self.counter.swap(0, Ordering::Relaxed);
        if counter > 0 {
            Some(format!(
                "[{}]: xruns: {}",
                Utc::now().format("%F %T"),
                counter
            ))
        } else {
            None
        }
    }

    fn print_output(&mut self) {
        match self.output() {
            None => {}
            Some(output) => {
                println!("{}", output);
            }
        }
    }
}

impl NotificationHandler for XRunLogger {
    fn xrun(&mut self, _: &Client) -> Control {
        self.xrun_();
        Control::Continue
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn output_returns_logged_xruns_as_message() {
        let mut logger = XRunLogger::new();
        logger.xrun_();
        assert!(logger.output().unwrap().ends_with("xruns: 1"));
    }

    #[test]
    fn output_includes_timestamp() {
        let mut logger = XRunLogger::new();
        logger.xrun_();
        let output = logger.output().unwrap();
        assert_eq!(output, Utc::now().format("[%F %T]: xruns: 1").to_string());
    }

    #[test]
    fn output_returns_none_when_no_xruns_were_logged() {
        let mut logger = XRunLogger::new();
        assert_eq!(logger.output(), None);
    }

    #[test]
    fn output_resets_the_xrun_counter() {
        let mut logger = XRunLogger::new();
        logger.xrun_();
        logger.output();
        assert_eq!(logger.output(), None);
    }

    fn run_in_thread<F, T>(mut action: F) -> T
    where
        T: Send + 'static,
        F: FnMut() -> T + Send + 'static,
    {
        ::std::thread::spawn(move || action()).join().unwrap()
    }

    #[test]
    fn output_can_be_invoked_from_another_thread() {
        let mut logger = XRunLogger::new();
        let mut logger_clone = logger.clone();
        logger.xrun_();
        assert!(
            run_in_thread(move || logger_clone.output())
                .unwrap()
                .ends_with("xruns: 1")
        );
        assert_eq!(logger.output(), None);
    }
}
