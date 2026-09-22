mod benchmark;
mod coordinator;
mod health_monitor;
mod task_queue;
mod types;
mod zone_control;

use std::time::Duration;
use std::{sync::Arc, thread};
use sysinfo::System;

use coordinator::Coordinator;
use health_monitor::HealthMonitor;
use task_queue::TaskQueue;
use types::{RobotStatus, RobotTask, TaskPriority};
use zone_control::ZoneAccessControl;

fn section(title: &str) {
    println!("\n==================== {title} ====================");
}

fn status_label(status: Option<RobotStatus>) -> &'static str {
    match status {
        Some(RobotStatus::Ready) => "Ready",
        Some(RobotStatus::Busy) => "Busy",
        Some(RobotStatus::Offline) => "Offline",
        None => "Unknown",
    }
}

fn run_worker(
    coordinator: Arc<Coordinator>,
    robot_id: &'static str,
    cycles: usize,
    work_ms: u64,
) -> usize {
    let mut completed = 0;

    for cycle in 1..=cycles {
        coordinator.report_heartbeat(robot_id);
        match coordinator.assign_next_task(robot_id) {
            Some(task) => {
                println!(
                    "WORKER  | {robot_id} | cycle {cycle} | ASSIGNED  | task #{} @ {}",
                    task.id, task.target_zone
                );
                thread::sleep(Duration::from_millis(work_ms));
                coordinator.complete_task(robot_id, &task);
                completed += 1;
                println!(
                    "WORKER  | {robot_id} | cycle {cycle} | COMPLETED | task #{}",
                    task.id
                );
            }
            None => {
                println!(
                    "WORKER  | {robot_id} | cycle {cycle} | SKIPPED   | queue empty or zone busy"
                );
                thread::sleep(Duration::from_millis(200));
            }
        }
    }

    completed
}

fn main() {
    let run_benchmark = std::env::args().any(|arg| arg == "benchmark");
    if run_benchmark {
        benchmark::run_default_benchmark();
        return;
    }

    let mut sys = System::new_all();
    let my_pid = sysinfo::get_current_pid().expect("failed to get process id");

    println!("Medical Care Robot Coordination Demo");
    section("SETUP");

    let queue = TaskQueue::new();
    let zones = ZoneAccessControl::new();
    let health = HealthMonitor::new(Duration::from_secs(2));
    let coordinator = Arc::new(Coordinator::new(
        queue.clone(),
        zones.clone(),
        health.clone(),
    ));

    let robot_names = ["robot-1", "robot-2", "robot-3"];
    for name in &robot_names {
        coordinator.register_robot(name);
    }
    println!("Robots registered : {}", robot_names.join(", "));

    let tasks = [
        (1, "ER", "Deliver emergency medicine", TaskPriority::Critical),
        (2, "ER", "Transport patient to imaging", TaskPriority::High),
        (3, "Ward-3", "Deliver IV kit", TaskPriority::High),
        (4, "Lab", "Collect blood sample", TaskPriority::Normal),
        (5, "Pharmacy", "Pick up medication", TaskPriority::Normal),
        (6, "ICU", "Deliver ventilator kit", TaskPriority::High),
        (7, "Ward-1", "Bring meal cart", TaskPriority::Normal),
        (8, "ER", "Return stretcher", TaskPriority::Normal),
        (9, "Radiology", "Move scan records", TaskPriority::Normal),
    ];

    for &(id, zone, desc, priority) in &tasks {
        coordinator.queue_task(RobotTask::new(id, zone, desc, priority));
    }

    println!("Initial task count: {}", queue.len());
    println!("Log format        : SOURCE | ROBOT | CYCLE/TICK | EVENT | DETAILS");
    section("LOGS");

    let worker_3 = {
        let coordinator = Arc::clone(&coordinator);
        thread::spawn(move || run_worker(coordinator, "robot-3", 3, 650))
    };

    thread::sleep(Duration::from_millis(120));

    let worker_1 = {
        let coordinator = Arc::clone(&coordinator);
        thread::spawn(move || run_worker(coordinator, "robot-1", 6, 850))
    };

    let worker_2 = {
        let coordinator = Arc::clone(&coordinator);
        thread::spawn(move || run_worker(coordinator, "robot-2", 6, 700))
    };

    for tick in 1..=6 {
        sys.refresh_processes();
        thread::sleep(Duration::from_secs(1));

        if let Some(process) = sys.process(my_pid) {
            let cpu = process.cpu_usage();
            let mem_mb = process.memory() / 1024 / 1024;
            println!(
                "MONITOR | system  | t={tick}s   | CPU: {cpu:>6.2}% | RAM: {mem_mb:>4} MB"
            );
        }

        coordinator.report_heartbeat("robot-1");
        coordinator.report_heartbeat("robot-2");

        let offline = coordinator.check_timeouts();
        if !offline.is_empty() {
            println!(
                "MONITOR | system  | t={tick}s   | TIMEOUT   | offline robots: {:?}",
                offline
            );
        } else {
            println!("MONITOR | system  | t={tick}s   | OK        | no robot timeout");
        }
    }

    let completed_1 = worker_1.join().expect("worker thread robot-1 panicked");
    let completed_2 = worker_2.join().expect("worker thread robot-2 panicked");
    let completed_3 = worker_3.join().expect("worker thread robot-3 panicked");

    coordinator.report_heartbeat("robot-1");
    coordinator.report_heartbeat("robot-2");

    section("FINAL STATE");
    println!("Queue length   : {}", queue.len());
    println!("Queue empty    : {}", queue.is_empty());
    println!("ER zone busy   : {}", zones.is_busy("ER"));
    println!(
        "jobs completed : robot-1={completed_1}, robot-2={completed_2}, robot-3={completed_3}"
    );

    for name in &robot_names {
        println!("{} status : {}", name, status_label(health.get_status(name)));
    }
}
