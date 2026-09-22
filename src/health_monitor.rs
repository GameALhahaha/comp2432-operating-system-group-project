use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::types::RobotStatus;

#[derive(Clone)]
pub struct HealthMonitor {
    timeout: Duration,
    robots: Arc<Mutex<HashMap<String, RobotHealth>>>,
}

#[derive(Clone, Debug)]
struct RobotHealth {
    status: RobotStatus,
    last_heartbeat: Instant,
}

impl HealthMonitor {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            robots: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_robot(&self, robot_id: &str) {
        let mut robots = self.robots.lock().unwrap();
        robots.insert(
            robot_id.to_string(),
            RobotHealth {
                status: RobotStatus::Ready,
                last_heartbeat: Instant::now(),
            },
        );
    }

    pub fn heartbeat(&self, robot_id: &str) {
        let mut robots = self.robots.lock().unwrap();
        if let Some(entry) = robots.get_mut(robot_id) {
            entry.last_heartbeat = Instant::now();
            if entry.status == RobotStatus::Offline {
                entry.status = RobotStatus::Ready;
            }
        }
    }

    pub fn set_status(&self, robot_id: &str, status: RobotStatus) {
        let mut robots = self.robots.lock().unwrap();
        if let Some(entry) = robots.get_mut(robot_id) {
            entry.status = status;
        }
    }

    pub fn get_status(&self, robot_id: &str) -> Option<RobotStatus> {
        let robots = self.robots.lock().unwrap();
        robots.get(robot_id).map(|entry| entry.status)
    }

    pub fn is_available(&self, robot_id: &str) -> bool {
        matches!(self.get_status(robot_id), Some(RobotStatus::Ready))
    }

    pub fn check_timeouts(&self) -> Vec<String> {
        let now = Instant::now();
        let mut robots = self.robots.lock().unwrap();
        let mut offline = Vec::new();

        for (robot_id, entry) in robots.iter_mut() {
            if now.duration_since(entry.last_heartbeat) > self.timeout
                && entry.status != RobotStatus::Offline
            {
                entry.status = RobotStatus::Offline;
                offline.push(robot_id.clone());
            }
        }

        offline
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;

    use super::HealthMonitor;
    use crate::types::RobotStatus;

    #[test]
    fn new_robot_is_ready_and_available() {
        let monitor = HealthMonitor::new(Duration::from_secs(1));
        monitor.register_robot("robot-1");

        assert_eq!(monitor.get_status("robot-1"), Some(RobotStatus::Ready));
        assert!(monitor.is_available("robot-1"));
    }

    #[test]
    fn timeout_marks_robot_offline_and_heartbeat_recovers() {
        let monitor = HealthMonitor::new(Duration::from_millis(10));
        monitor.register_robot("robot-2");

        thread::sleep(Duration::from_millis(20));
        let offline = monitor.check_timeouts();
        assert_eq!(offline, vec!["robot-2".to_string()]);
        assert_eq!(monitor.get_status("robot-2"), Some(RobotStatus::Offline));

        monitor.heartbeat("robot-2");
        assert_eq!(monitor.get_status("robot-2"), Some(RobotStatus::Ready));
    }
}
