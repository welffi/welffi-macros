use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Clone, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn is_set(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }

    pub fn nap(&self, ms: u64) -> bool {
        let until = Instant::now() + Duration::from_millis(ms);
        while Instant::now() < until {
            if self.is_set() {
                return false;
            }
            thread::sleep(Duration::from_millis(10).min(until - Instant::now()));
        }
        !self.is_set()
    }
}

pub struct Worker {
    cancel: Cancel,
    handle: JoinHandle<()>,
}

impl Worker {
    pub fn spawn(job: impl FnOnce(Cancel) + Send + 'static) -> Worker {
        let cancel = Cancel::default();
        let for_job = cancel.clone();
        Worker { cancel, handle: thread::spawn(move || job(for_job)) }
    }

    pub fn running(&self) -> bool {
        !self.handle.is_finished()
    }

    pub fn stop(&self) {
        self.cancel.0.store(true, Ordering::Relaxed);
    }
}

pub fn is_running(worker: &Option<Worker>) -> bool {
    worker.as_ref().is_some_and(Worker::running)
}
