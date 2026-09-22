use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use sysinfo::{System, Pid};

use crate::coordinator::Coordinator;
use crate::health_monitor::HealthMonitor;
use crate::task_queue::TaskQueue;
use crate::types::{RobotTask, TaskPriority, RobotStatus};
use crate::zone_control::ZoneAccessControl;

pub struct BenchmarkResult {
	pub robot_count: usize,
	pub task_count: usize,
	pub tasks_completed: usize,
	pub elapsed_secs: f64,
	pub throughput: f64,
	pub queue_remaining: usize,
	pub avg_zone_latency_ms: f64,
	pub max_zone_latency_ms: f64,
	pub cpu_percent: f64,
	pub memory_mb: f64,
}

pub struct ZoneLatencyTracker {
	latencies: Arc<AtomicUsize>,
	max_latency: Arc<AtomicUsize>,
	count: Arc<AtomicUsize>,
}

impl ZoneLatencyTracker {
	pub fn new() -> Self {
		Self {
			latencies: Arc::new(AtomicUsize::new(0)),
			max_latency: Arc::new(AtomicUsize::new(0)),
			count: Arc::new(AtomicUsize::new(0)),
		}
	}

	pub fn record(&self, latency_us: usize) {
		self.latencies.fetch_add(latency_us, Ordering::Relaxed);
		self.count.fetch_add(1, Ordering::Relaxed);
		let mut max = self.max_latency.load(Ordering::Relaxed);
		while latency_us > max {
			match self.max_latency.compare_exchange(
				max,
				latency_us,
				Ordering::Release,
				Ordering::Relaxed,
			) {
				Ok(_) => break,
				Err(actual) => max = actual,
			}
		}
	}

	pub fn get_avg_us(&self) -> usize {
		let count = self.count.load(Ordering::Relaxed);
		if count == 0 {
			0
		} else {
			self.latencies.load(Ordering::Relaxed) / count
		}
	}

	pub fn get_max_us(&self) -> usize {
		self.max_latency.load(Ordering::Relaxed)
	}
}

pub struct ZoneContentionResult {
	pub duration_secs: u64,
	pub total_attempts: usize,
	pub successful_entries: usize,
	pub rejected_entries: usize,
}

pub struct TimeoutDetectionResult {
	pub timeout_secs: u64,
	pub robots_registered: usize,
	pub robots_offline_detected: usize,
}

pub fn run_simple_benchmark(robot_count: usize, task_count: usize) -> BenchmarkResult {
	let queue = TaskQueue::new();
	let zones = ZoneAccessControl::new();
	let health = HealthMonitor::new(Duration::from_secs(30));
	let coordinator = Arc::new(Coordinator::new(
		queue.clone(),
		zones.clone(),
		health.clone(),
	));

	let latency_tracker = Arc::new(ZoneLatencyTracker::new());
	let robot_ids: Vec<String> = (1..=robot_count).map(|i| format!("robot-{i}")).collect();

	for robot_id in &robot_ids {
		coordinator.register_robot(robot_id);
	}

	let zones_list = ["ER", "ICU", "Lab", "Pharmacy", "Ward-1", "Ward-2"];
	for i in 1..=task_count {
		let zone = zones_list[i % zones_list.len()];
		let priority = match i % 10 {
			0 => TaskPriority::Critical,
			1 | 2 => TaskPriority::High,
			_ => TaskPriority::Normal,
		};
		coordinator.queue_task(RobotTask::new(i, zone, "benchmark-task", priority));
	}

	// Get initial system metrics
	let mut sys = System::new_all();
	sys.refresh_all();
	let pid = Pid::from(std::process::id() as usize);
	let process = sys.process(pid).unwrap();
	let initial_cpu = process.cpu_usage() as f64;
	let initial_memory = process.memory();

	let started = Instant::now();
	let mut workers = Vec::with_capacity(robot_ids.len());

	for robot_id in robot_ids {
		let c = Arc::clone(&coordinator);
		let q = queue.clone();
		let tracker = Arc::clone(&latency_tracker);

		workers.push(thread::spawn(move || {
			let mut completed = 0usize;
			let mut idle_spins = 0usize;

			loop {
				if idle_spins % 10 == 0 {
					c.report_heartbeat(&robot_id);
				}

				let zone_start = Instant::now();
				if let Some(task) = c.assign_next_task(&robot_id) {
					let latency_us = zone_start.elapsed().as_micros() as usize;
					tracker.record(latency_us);

					c.complete_task(&robot_id, &task);
					completed += 1;
					idle_spins = 0;
				} else {
					idle_spins += 1;
					if q.is_empty() && idle_spins > 10 {
						break;
					}
					thread::yield_now();
				}
			}

			completed
		}));
	}

	let total_completed: usize = workers
		.into_iter()
		.map(|w| w.join().expect("worker thread panicked"))
		.sum();

	let elapsed = started.elapsed();
	let seconds = elapsed.as_secs_f64();
	let throughput = if seconds > 0.0 {
		total_completed as f64 / seconds
	} else {
		0.0
	};

	// Get final system metrics
	sys.refresh_all();
	let process = sys.process(pid).unwrap();
	let final_cpu = process.cpu_usage() as f64;
	let final_memory = process.memory();

	let avg_latency_us = latency_tracker.get_avg_us();
	let max_latency_us = latency_tracker.get_max_us();

	BenchmarkResult {
		robot_count,
		task_count,
		tasks_completed: total_completed,
		elapsed_secs: seconds,
		throughput,
		queue_remaining: queue.len(),
		avg_zone_latency_ms: avg_latency_us as f64 / 1000.0,
		max_zone_latency_ms: max_latency_us as f64 / 1000.0,
		cpu_percent: final_cpu - initial_cpu,
		memory_mb: (final_memory - initial_memory) as f64 / 1024.0,
	}
}

pub fn run_zone_contention_stress_test(duration_secs: u64) -> ZoneContentionResult {
	let zones = ZoneAccessControl::new();
	let zone_contested = Arc::new(AtomicUsize::new(0));
	let zone_acquired = Arc::new(AtomicUsize::new(0));

	let zones_clones = (0..4).map(|_| zones.clone()).collect::<Vec<_>>();
	let contested_clones = (0..4).map(|_| Arc::clone(&zone_contested)).collect::<Vec<_>>();
	let acquired_clones = (0..4).map(|_| Arc::clone(&zone_acquired)).collect::<Vec<_>>();

	let started = Instant::now();
	let mut handles = vec![];

	for i in 0..4 {
		let z = zones_clones[i].clone();
		let contested = contested_clones[i].clone();
		let acquired = acquired_clones[i].clone();

		handles.push(thread::spawn(move || {
			while started.elapsed().as_secs() < duration_secs {
				if z.try_enter("shared_zone") {
					acquired.fetch_add(1, Ordering::Relaxed);
					thread::sleep(Duration::from_millis(10));
					z.leave("shared_zone");
				} else {
					contested.fetch_add(1, Ordering::Relaxed);
				}
			}
		}));
	}

	for h in handles {
		h.join().unwrap();
	}

	ZoneContentionResult {
		duration_secs,
		total_attempts: zone_acquired.load(Ordering::Relaxed) + zone_contested.load(Ordering::Relaxed),
		successful_entries: zone_acquired.load(Ordering::Relaxed),
		rejected_entries: zone_contested.load(Ordering::Relaxed),
	}
}

pub fn run_timeout_stress_test(timeout_secs: u64) -> TimeoutDetectionResult {
	let health = HealthMonitor::new(Duration::from_secs(timeout_secs));
	let robot_ids: Vec<String> = (1..=3).map(|i| format!("robot-{i}")).collect();

	for id in &robot_ids {
		health.register_robot(id);
	}

	// Simulate robots, send initial heartbeats
	for id in &robot_ids {
		health.heartbeat(id);
		health.set_status(id, RobotStatus::Busy);
	}

	// Sleep through timeout period
	thread::sleep(Duration::from_secs(timeout_secs + 2));

	// Manually trigger timeout check
	let offline_robots = health.check_timeouts();
	let offline_count = offline_robots.len();

	TimeoutDetectionResult {
		timeout_secs,
		robots_registered: robot_ids.len(),
		robots_offline_detected: offline_count,
	}
}

pub fn run_default_benchmark() {
	let args: Vec<String> = std::env::args().collect();

	let csv_mode = args.iter().any(|a| a == "--csv");
	let skip_stress = args.iter().any(|a| a == "--no-stress");

	let robot_counts: Vec<usize> = if args.len() > 2 && !args[2].starts_with('-') {
		args[2].split(',').filter_map(|s| s.parse().ok()).collect()
	} else {
		vec![1, 2, 4, 8, 16]
	};

	let task_counts: Vec<usize> = if args.len() > 3 && !args[3].starts_with('-') {
		args[3].split(',').filter_map(|s| s.parse().ok()).collect()
	} else {
		vec![1000, 5000, 10000, 20000, 50000, 100000]
	};

	let runs: usize = args
		.iter()
		.position(|a| a == "--runs")
		.and_then(|i| args.get(i + 1))
		.and_then(|s| s.parse().ok())
		.unwrap_or(3);

	if csv_mode {
		println!("robots,tasks,run,completed,elapsed_s,throughput_tasks_per_s,queue_remaining,avg_zone_latency_ms,max_zone_latency_ms,cpu_percent,memory_mb");
	}

	for &rc in &robot_counts {
		for &tc in &task_counts {
			let mut best_throughput = 0.0f64;
			let mut best_result: Option<BenchmarkResult> = None;

			for run in 1..=runs {
				let result = run_simple_benchmark(rc, tc);

				if csv_mode {
					println!(
						"{},{},{},{},{:.6},{:.2},{},{:.4},{:.4},{:.2},{:.2}",
						rc,
						tc,
						run,
						result.tasks_completed,
						result.elapsed_secs,
						result.throughput,
						result.queue_remaining,
						result.avg_zone_latency_ms,
						result.max_zone_latency_ms,
						result.cpu_percent,
						result.memory_mb,
					);
				}

				if result.throughput > best_throughput {
					best_throughput = result.throughput;
					best_result = Some(result);
				}
			}

			if !csv_mode {
				if let Some(r) = best_result {
					println!("\n=== Benchmark Result (best of {runs}) ===");
					println!("Concurrency:");
					println!("  robots          : {}", r.robot_count);
					println!("  tasks queued    : {}", r.task_count);
					println!("\nPerformance:");
					println!("  tasks completed : {}", r.tasks_completed);
					println!("  elapsed         : {:.3} s", r.elapsed_secs);
					println!("  throughput      : {:.2} tasks/s", r.throughput);
					println!("\nResource Usage:");
					println!("  zone latency avg: {:.4} ms", r.avg_zone_latency_ms);
					println!("  zone latency max: {:.4} ms", r.max_zone_latency_ms);
					println!("  CPU usage       : {:.2}%", r.cpu_percent);
					println!("  Memory delta    : {:.2} MB", r.memory_mb);
					println!("  queue remaining : {}", r.queue_remaining);
				}
			}
		}
	}

	// Run stress tests
	if !csv_mode && !skip_stress {
		println!("\n\n=== STRESS TESTS ===\n");
		
		println!("--- Zone Contention Stress Test ---");
		let zone_result = run_zone_contention_stress_test(5);
		println!("Duration: {} seconds", zone_result.duration_secs);
		println!("Total attempts: {}", zone_result.total_attempts);
		println!("Successful entries: {}", zone_result.successful_entries);
		println!("Rejected (contended): {}", zone_result.rejected_entries);
		if zone_result.total_attempts > 0 {
			println!("Success rate: {:.2}%", 
					 (zone_result.successful_entries as f64 / zone_result.total_attempts as f64) * 100.0);
		}

		println!("\n--- Timeout Detection Stress Test ---");
		let timeout_result = run_timeout_stress_test(3);
		println!("Timeout threshold: {} seconds", timeout_result.timeout_secs);
		println!("Robots registered: {}", timeout_result.robots_registered);
		println!("Robots detected offline: {}", timeout_result.robots_offline_detected);
	}
}
