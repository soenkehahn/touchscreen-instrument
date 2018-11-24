use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone)]
pub struct Quitter {
    mutex: Arc<Mutex<Internal>>,
}

struct Internal {
    handlers: Vec<Box<FnMut() + Send>>,
    exit: Box<FnMut() + Send>,
}

impl Quitter {
    pub fn new() -> Quitter {
        Quitter::new_internal(|| ::std::process::exit(0))
    }

    fn new_internal<F>(exit: F) -> Quitter
    where
        F: FnMut() + 'static + Send,
    {
        Quitter {
            mutex: Arc::new(Mutex::new(Internal {
                handlers: vec![],
                exit: Box::new(exit),
            })),
        }
    }

    fn lock(&self) -> MutexGuard<Internal> {
        self.mutex.lock().expect("Quitter mutex poisoned")
    }

    pub fn register_cleanup<F>(&mut self, handler: F)
    where
        F: FnMut() + 'static + Send,
    {
        let mut internal = self.lock();
        internal.handlers.push(Box::new(handler));
    }

    pub fn quit(&self) {
        let mut internal = self.lock();
        for mut handler in internal.handlers.iter_mut() {
            handler();
        }
        internal.exit.as_mut()();
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Clone)]
    struct Tracker {
        foo: Arc<Mutex<Vec<String>>>,
    }

    impl Tracker {
        fn new() -> Tracker {
            let actions: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
            Tracker { foo: actions }
        }

        fn log(&mut self, message: &str) {
            let mut vec = self.foo.lock().unwrap();
            vec.push(String::from(message));
        }

        fn get_logs(&self) -> Vec<String> {
            self.foo.lock().unwrap().clone()
        }
    }

    #[test]
    fn runs_registered_cleanup_handlers() {
        let mut quitter = Quitter::new_internal(|| {});
        let tracker = Tracker::new();
        let mut tracker_clone = tracker.clone();
        quitter.register_cleanup(move || {
            tracker_clone.log("handler");
        });
        quitter.quit();
        assert_eq!(tracker.get_logs(), vec!["handler"]);
    }

    #[test]
    fn runs_process_exit() {
        let tracker = Tracker::new();
        let mut tracker_clone = tracker.clone();
        let quit = move || {
            tracker_clone.log("quit");
        };
        let quitter = Quitter::new_internal(quit);
        quitter.quit();
        assert_eq!(tracker.get_logs(), vec!["quit"]);
    }

    #[test]
    fn runs_cleanup_handlers_before_running_exit() {
        let tracker = Tracker::new();
        let mut tracker_clone_1 = tracker.clone();
        let quit = move || {
            tracker_clone_1.log("quit");
        };
        let mut quitter = Quitter::new_internal(quit);
        let mut tracker_clone_2 = tracker.clone();
        quitter.register_cleanup(move || tracker_clone_2.log("handler"));
        quitter.quit();
        assert_eq!(tracker.get_logs(), vec!["handler", "quit"]);
    }

    #[test]
    fn supports_multiple_cleanup_handlers() {
        let tracker = Tracker::new();
        let mut quitter = Quitter::new_internal(|| {});
        let mut tracker_clone_1 = tracker.clone();
        quitter.register_cleanup(move || tracker_clone_1.log("handler 1"));
        let mut tracker_clone_2 = tracker.clone();
        quitter.register_cleanup(move || tracker_clone_2.log("handler 2"));
        quitter.quit();
        assert_eq!(tracker.get_logs(), vec!["handler 1", "handler 2"]);
    }

    #[test]
    fn closed_over_values_in_cleanup_handlers_do_not_have_to_implement_clone() {
        #[derive(Debug)]
        struct Foo;
        let foo = Foo;
        let mut quitter = Quitter::new_internal(|| {});
        quitter.register_cleanup(move || println!("{:?}", foo));
    }
}
