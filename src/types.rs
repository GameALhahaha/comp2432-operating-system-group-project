#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RobotStatus {
    Ready,
    Busy,
    Offline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    Critical,
    High,
    Normal,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RobotTask {
    pub id: usize,
    pub target_zone: String,
    pub description: String,
    pub priority: TaskPriority,
}

impl RobotTask {
    pub fn new(id: usize, target_zone: &str, description: &str, priority: TaskPriority) -> Self {
        RobotTask {
            id,
            target_zone: target_zone.to_string(),
            description: description.to_string(),
            priority,
        }
    }
}
