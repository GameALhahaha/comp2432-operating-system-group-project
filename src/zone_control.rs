use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct ZoneAccessControl {
    busy_zones: Arc<Mutex<HashSet<String>>>,
}

impl ZoneAccessControl {
    pub fn new() -> Self {
        ZoneAccessControl {
            busy_zones: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn try_enter(&self, zone: &str) -> bool {
        let mut zones = self.busy_zones.lock().unwrap();
        zones.insert(zone.to_string())
    }

    pub fn leave(&self, zone: &str) {
        let mut zones = self.busy_zones.lock().unwrap();
        zones.remove(zone);
    }

    pub fn is_busy(&self, zone: &str) -> bool {
        let zones = self.busy_zones.lock().unwrap();
        zones.contains(zone)
    }
}

#[cfg(test)]
mod tests {
    use super::ZoneAccessControl;

    #[test]
    fn enter_and_leave_zone() {
        let zones = ZoneAccessControl::new();
        assert!(zones.try_enter("ER"));
        assert!(zones.is_busy("ER"));

        zones.leave("ER");
        assert!(!zones.is_busy("ER"));
    }

    #[test]
    fn duplicate_enter_is_rejected_until_leave() {
        let zones = ZoneAccessControl::new();
        assert!(zones.try_enter("Lab"));
        assert!(!zones.try_enter("Lab"));

        zones.leave("Lab");
        assert!(zones.try_enter("Lab"));
    }
}
