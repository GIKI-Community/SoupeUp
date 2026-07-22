use std::collections::VecDeque;
use crate::runtime::task::Task;

pub struct TaskQueue {
    queue: VecDeque<Task>,
}

impl TaskQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, task: Task) {
        self.queue.push_back(task);
    }

    pub fn pop(&mut self) -> Option<Task> {
        self.queue.pop_front()
    }
}
