use crate::runtime::queue::TaskQueue;
use crate::runtime::worker::Worker;

pub struct Scheduler {
    queue: TaskQueue,
    workers: Vec<Worker>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            queue: TaskQueue::new(),
            workers: Vec::new(),
        }
    }

    pub fn schedule_next(&mut self) {
        // Find idle worker, pop task, assign.
    }
}
