use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::types::{RobotTask, TaskPriority};

type SafeQueue = Arc<Mutex<VecDeque<RobotTask>>>;
#[derive(Clone)]
pub struct TaskQueue {
    critical: SafeQueue,
    high: SafeQueue,
    normal: SafeQueue,
}

impl TaskQueue {
    pub fn new() -> Self {
        TaskQueue {
            critical: Arc::new(Mutex::new(VecDeque::new())),
            high: Arc::new(Mutex::new(VecDeque::new())),
            normal: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn push_task(&self, task: RobotTask) {
        match task.priority {
            TaskPriority::Critical => {
                let mut queue = self.critical.lock().unwrap();
                queue.push_back(task);
            }
            TaskPriority::High => {
                let mut queue = self.high.lock().unwrap();
                queue.push_back(task);
            }
            TaskPriority::Normal => {
                let mut queue = self.normal.lock().unwrap();
                queue.push_back(task);
            }
        };
    }

    pub fn fetch_task(&self) -> Option<RobotTask> {
        if let Some(task) = self.critical.lock().unwrap().pop_front() {
            return Some(task);
        }
        if let Some(task) = self.high.lock().unwrap().pop_front() {
            return Some(task);
        }
        self.normal.lock().unwrap().pop_front()
    }

    pub fn len(&self) -> usize {
        let critical = self.critical.lock().unwrap();
        let high = self.high.lock().unwrap();
        let normal = self.normal.lock().unwrap();
        critical.len() + high.len() + normal.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::TaskQueue;
    use crate::types::{RobotTask, TaskPriority};

    #[test]
    fn fetches_higher_priority_tasks_first() {
        let queue = TaskQueue::new();
        queue.push_task(RobotTask::new(1, "Ward-1", "normal", TaskPriority::Normal));
        queue.push_task(RobotTask::new(2, "ER", "critical", TaskPriority::Critical));
        queue.push_task(RobotTask::new(3, "Lab", "high", TaskPriority::High));

        assert_eq!(queue.fetch_task().map(|t| t.id), Some(2));
        assert_eq!(queue.fetch_task().map(|t| t.id), Some(3));
        assert_eq!(queue.fetch_task().map(|t| t.id), Some(1));
        assert_eq!(queue.fetch_task(), None);
    }

    #[test]
    fn tracks_length_and_empty_state() {
        let queue = TaskQueue::new();
        assert!(queue.is_empty());

        queue.push_task(RobotTask::new(10, "ICU", "deliver kit", TaskPriority::High));
        queue.push_task(RobotTask::new(11, "ER", "triage", TaskPriority::Critical));
        assert_eq!(queue.len(), 2);
        assert!(!queue.is_empty());

        let _ = queue.fetch_task();
        let _ = queue.fetch_task();
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
    }
}
