use chrono::prelude::*;
use jack::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone)]
pub struct Logger {
    counter: Arc<AtomicUsize>,
    clipping: Arc<AtomicBool>,
}

impl Logger {
    pub fn new_and_spawn() -> Logger {
        let result = Logger::new();
        let mut clone = result.clone();
        ::std::thread::spawn(move || loop {
            ::std::thread::sleep(::std::time::Duration::new(1, 0));
            clone.print_output();
        });
        result
    }

    fn new() -> Logger {
        Logger {
            counter: Arc::new(AtomicUsize::new(0)),
            clipping: Arc::new(AtomicBool::new(false)),
        }
    }

    fn log_xrun(&mut self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
    }

    fn log_clipping(&self) {
        self.clipping.store(true, Ordering::Relaxed);
    }

    pub fn check_clipping(&self, buffer: &[f32]) {
        for sample in buffer.iter() {
            if *sample > 1.0 || *sample < -1.0 {
                self.log_clipping();
            }
        }
    }

    fn output(&mut self) -> Option<String> {
        let mut messages = vec![];
        let counter = self.counter.swap(0, Ordering::Relaxed);
        if counter > 0 {
            messages.push(format!("xruns: {}", counter));
        }
        let clipping = self.clipping.swap(false, Ordering::Relaxed);
        if clipping {
            messages.push("output was clipped".to_string());
        }
        if !messages.is_empty() {
            Some(format!(
                "[{}]: {}",
                Utc::now().format("%F %T"),
                messages.join(", ")
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

impl NotificationHandler for Logger {
    fn xrun(&mut self, _: &Client) -> Control {
        self.log_xrun();
        Control::Continue
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn output_includes_timestamp() {
        let mut logger = Logger::new();
        logger.log_xrun();
        let output = logger.output().unwrap();
        assert_eq!(output, Utc::now().format("[%F %T]: xruns: 1").to_string());
    }

    #[test]
    fn output_returns_nothing_was_logged() {
        let mut logger = Logger::new();
        assert_eq!(logger.output(), None);
    }

    mod xrun_logging {
        use super::*;

        #[test]
        fn output_returns_logged_xruns_as_message() {
            let mut logger = Logger::new();
            logger.log_xrun();
            assert!(logger.output().unwrap().ends_with("xruns: 1"));
        }

        #[test]
        fn output_resets_the_xrun_counter() {
            let mut logger = Logger::new();
            logger.log_xrun();
            logger.output();
            assert_eq!(logger.output(), None);
        }
    }

    mod clipping_logging {
        use super::*;

        #[test]
        fn output_returns_logged_clipping_as_message() {
            let mut logger = Logger::new();
            logger.log_clipping();
            let output = logger.output().unwrap();
            assert!(output.ends_with("]: output was clipped"), output);
        }

        #[test]
        fn output_resets_the_clipping_flag() {
            let mut logger = Logger::new();
            logger.log_clipping();
            logger.output();
            assert_eq!(logger.output(), None);
        }

        #[test]
        fn output_combines_xruns_and_clipping() {
            let mut logger = Logger::new();
            logger.log_xrun();
            logger.log_clipping();
            let output = logger.output().unwrap();
            assert!(output.ends_with("]: xruns: 1, output was clipped"), output);
        }

        mod check_clipping {
            use super::*;

            #[test]
            fn logs_clipping_in_buffers() {
                let mut logger = Logger::new();
                logger.check_clipping(&[1.1][..]);
                let output = logger.output().unwrap();
                assert!(output.ends_with("]: output was clipped"), output);
            }

            #[test]
            fn detects_negative_clipping() {
                let mut logger = Logger::new();
                logger.check_clipping(&[-1.1][..]);
                let output = logger.output().unwrap();
                assert!(output.ends_with("]: output was clipped"), output);
            }

            #[test]
            fn detects_non_clipping_buffers() {
                let mut logger = Logger::new();
                logger.check_clipping(&[0.0][..]);
                assert_eq!(logger.output(), None);
            }
        }
    }

    mod thread_behavior {
        use super::*;

        fn run_in_thread<F, T>(mut action: F) -> T
        where
            T: Send + 'static,
            F: FnMut() -> T + Send + 'static,
        {
            ::std::thread::spawn(move || action()).join().unwrap()
        }

        #[test]
        fn output_can_be_invoked_from_another_thread() {
            let mut logger = Logger::new();
            let mut logger_clone = logger.clone();
            logger.log_xrun();
            logger.log_clipping();
            let output = run_in_thread(move || logger_clone.output()).unwrap();
            assert!(output.ends_with("xruns: 1, output was clipped"), output);
            assert_eq!(logger.output(), None);
        }
    }
}
