use std::collections::VecDeque;

use crate::jobs::models::{Job, JobStatus};

pub struct JobQueue {
    pending: VecDeque<String>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self {
            pending: VecDeque::new(),
        }
    }

    pub fn enqueue(&mut self, job_id: &str) {
        self.pending.push_back(job_id.to_string());
    }

    pub fn dequeue(&mut self) -> Option<String> {
        self.pending.pop_front()
    }

    pub fn remove(&mut self, job_id: &str) {
        self.pending.retain(|id| id != job_id);
    }

    pub fn len(&self) -> usize {
        self.pending.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pending.is_empty()
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

pub fn can_transition(from: JobStatus, to: JobStatus) -> bool {
    use JobStatus::*;
    matches!(
        (from, to),
        (Created, Queued)
            | (Queued, Scheduling)
            | (Scheduling, Running)
            | (Running, Completed)
            | (Running, Failed)
            | (Running, Cancelled)
            | (Created, Cancelled)
            | (Queued, Cancelled)
            | (Scheduling, Cancelled)
            | (Failed, Queued)
    )
}

pub fn transition(job: &mut Job, to: JobStatus) -> bool {
    if can_transition(job.status.clone(), to.clone()) {
        job.status = to;
        true
    } else if job.status == to {
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_fifo() {
        let mut q = JobQueue::new();
        q.enqueue("a");
        q.enqueue("b");
        assert_eq!(q.dequeue(), Some("a".into()));
        assert_eq!(q.dequeue(), Some("b".into()));
    }

    #[test]
    fn lifecycle_transitions() {
        use JobStatus::*;
        assert!(can_transition(Created, Queued));
        assert!(can_transition(Running, Completed));
        assert!(!can_transition(Completed, Running));
    }
}
