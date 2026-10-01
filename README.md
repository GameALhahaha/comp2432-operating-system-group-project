# COMP2432 Group Project

Group members: Lu Chi Ngai, FENG Cheong Hoi, LAW Ka Chun, YAU Kwin Yue

A Rust simulation for medical care robot coordination, including:

- Priority task queue (Critical, High, Normal)
- Zone access control to prevent zone conflicts
- Robot health monitoring with heartbeat timeout detection
- Coordinator for task assignment/completion
- Benchmark mode for throughput measurements

## Project Layout

```
src/
	main.rs             # demo entry
	benchmark.rs        # benchmark runner
	coordinator.rs      # orchestration logic
	health_monitor.rs   # robot health and timeout checks
	task_queue.rs       # priority queues
	zone_control.rs     # zone lock control
	types.rs            # shared enums and task struct
bench_graph/          # benchmark output and chart script
```

## Run

Run simulation demo:

```bash
cargo run
```

Run benchmark with default matrix:

```bash
cargo run -- benchmark
```

Run benchmark with CSV output:

```bash
cargo run -- benchmark --csv
```

Run benchmark with custom robot/task sets and runs:

```bash
cargo run -- benchmark 1,2,4,8 1000,5000,10000 --runs 5 --csv
```

## Test

```bash
cargo test
```
