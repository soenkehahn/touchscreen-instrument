use skipchannel::*;
use std::thread::{sleep, spawn, JoinHandle};
use std::time::Duration;

enum Message<T> {
    Input(T),
    StopThread,
}

pub struct ThreadWorker<Input: Send, Output: Send> {
    thread: Option<JoinHandle<()>>,
    to_worker_thread: Sender<Message<Input>>,
    from_worker_thread: Receiver<Output>,
}

impl<Input: Send, Output: Send> Drop for ThreadWorker<Input, Output> {
    fn drop(&mut self) {
        self.to_worker_thread.send(Message::StopThread);
        let thread = std::mem::replace(&mut self.thread, None);
        match thread {
            None => {}
            Some(join_handle) => {
                match join_handle.join() {
                    Ok(()) => {}
                    Err(error) => eprintln!("{:?}", error),
                };
            }
        }
    }
}

impl<Input, Output> ThreadWorker<Input, Output>
where
    Input: Send + 'static,
    Output: Send + Clone + 'static,
{
    pub fn new<F>(mut computation: F) -> ThreadWorker<Input, Output>
    where
        F: FnMut(Input) -> Output + Send + 'static,
    {
        let (to_worker_thread, worker_thread_source) = skipchannel();
        let (worker_thread_sink, from_worker_thread) = skipchannel();
        let thread = spawn(move || loop {
            match worker_thread_source.recv() {
                None => {}
                Some(Message::Input(input)) => {
                    let result = computation(input);
                    worker_thread_sink.send(result);
                }
                Some(Message::StopThread) => break,
            };
            sleep(Duration::from_millis(100));
        });
        ThreadWorker {
            thread: Some(thread),
            to_worker_thread,
            from_worker_thread,
        }
    }

    pub fn enqueue(&self, input: Input) {
        self.to_worker_thread.send(Message::Input(input));
    }

    pub fn poll(&self) -> Option<Output> {
        self.from_worker_thread.recv()
    }
}

#[cfg(test)]
pub mod test {
    use super::*;
    use std::thread::ThreadId;

    pub fn wait_for<T, F>(mut f: F) -> Result<T, String>
    where
        F: FnMut() -> Result<T, String>,
    {
        let mut milli_seconds_left = 1000;
        let mut result = f();
        while milli_seconds_left > 0 && result.is_err() {
            result = f();
            sleep(Duration::from_millis(10));
            milli_seconds_left -= 10;
        }
        result
    }

    #[test]
    fn allows_to_run_functions() -> Result<(), String> {
        let thread_worker: ThreadWorker<i32, i32> = ThreadWorker::new(|x| x * 2);
        thread_worker.enqueue(42);
        let result = wait_for(|| match thread_worker.poll() {
            None => Err("poll: no result received".to_string()),
            Some(x) => Ok(x),
        })?;
        assert_eq!(result, 84);
        Ok(())
    }

    #[test]
    fn runs_computations_in_different_thread() -> Result<(), String> {
        let thread_worker: ThreadWorker<(), ThreadId> =
            ThreadWorker::new(|()| std::thread::current().id());
        thread_worker.enqueue(());
        let result = wait_for(|| match thread_worker.poll() {
            None => Err("poll: no result received".to_string()),
            Some(x) => Ok(x),
        })?;
        assert_ne!(result, std::thread::current().id());
        Ok(())
    }
}
