use crate::health_monitor::HealthMonitor;
use crate::task_queue::TaskQueue;
use crate::types::{RobotStatus, RobotTask};
use crate::zone_control::ZoneAccessControl;

pub struct Coordinator {
    queue: TaskQueue,
    zones: ZoneAccessControl,
    health: HealthMonitor,
}

impl Coordinator {
    pub fn new(queue: TaskQueue, zones: ZoneAccessControl, health: HealthMonitor) -> Self {
        Self {
            queue,
            zones,
            health,
        }
    }

    pub fn register_robot(&self, robot_id: &str) {
        self.health.register_robot(robot_id);
    }

    pub fn report_heartbeat(&self, robot_id: &str) {
        self.health.heartbeat(robot_id);
    }

    pub fn queue_task(&self, task: RobotTask) {
        self.queue.push_task(task);
    }

    pub fn assign_next_task(&self, robot_id: &str) -> Option<RobotTask> {
        if !self.health.is_available(robot_id) {
            return None;
        }

        let task = self.queue.fetch_task()?;

        if self.zones.try_enter(&task.target_zone) {
            self.health.set_status(robot_id, RobotStatus::Busy);
            Some(task)
        } else {
            self.queue.push_task(task);
            None
        }
    }

    pub fn complete_task(&self, robot_id: &str, task: &RobotTask) {
        self.zones.leave(&task.target_zone);
        self.health.set_status(robot_id, RobotStatus::Ready);
    }

    pub fn check_timeouts(&self) -> Vec<String> {
        self.health.check_timeouts()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::Coordinator;
    use crate::health_monitor::HealthMonitor;
    use crate::task_queue::TaskQueue;
    use crate::types::{RobotTask, TaskPriority};
    use crate::zone_control::ZoneAccessControl;

    #[test]
    fn second_robot_waits_when_zone_is_busy_then_gets_task_after_release() {
        let queue = TaskQueue::new();
        let zones = ZoneAccessControl::new();
        let health = HealthMonitor::new(Duration::from_secs(1));
        let coordinator = Coordinator::new(queue.clone(), zones, health);

        coordinator.register_robot("r1");
        coordinator.register_robot("r2");

        coordinator.queue_task(RobotTask::new(1, "ER", "first", TaskPriority::High));
        coordinator.queue_task(RobotTask::new(2, "ER", "second", TaskPriority::High));

        let first = coordinator
            .assign_next_task("r1")
            .expect("r1 should receive first ER task");
        assert_eq!(first.id, 1);

        // The second task targets the same zone and should be re-queued.
        assert!(coordinator.assign_next_task("r2").is_none());
        assert_eq!(queue.len(), 1);

        coordinator.complete_task("r1", &first);

        let second = coordinator
            .assign_next_task("r2")
            .expect("r2 should receive re-queued ER task after release");
        assert_eq!(second.id, 2);
    }
}
