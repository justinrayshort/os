---
title: "Performance Engineering and Benchmarking Strategy"
category: "reference"
owner: "platform-team"
status: "active"
last_reviewed: "2026-02-26"
audience: ["engineering", "platform"]
invariants:
  - "Performance changes are validated with repeatable measurements before and after optimization."
  - "Functional correctness tests must pass before and after benchmark or profiling-driven changes."
tags: ["reference", "performance", "benchmarking", "profiling"]
domain: "runtime"
lifecycle: "ga"
---

# Performance Engineering and Benchmarking Strategy

This document defines the project-wide performance engineering reference model: what to measure, how to measure it, and how to evaluate regressions consistently across the Rust workspace.

Use this page as the neutral reference for performance tooling, baselines, thresholds, artifact locations, and coverage expectations. Use the SOP for the procedural workflow and the wiki explanation/how-to pages for rationale and task execution.

## Objectives

- Keep optimization work data-driven and repeatable.
- Preserve functional correctness while improving latency, throughput, memory efficiency, and startup behavior.
- Detect regressions early in local development and in controlled benchmark environments.
- Document tradeoffs when performance changes affect readability, maintainability, or portability.

## Required Measurement Principles

- Run correctness checks before and after performance changes (`cargo perf check` at minimum for performance-sensitive work).
- Capture a baseline before optimization (`cargo perf baseline <name>` when Criterion benches exist).
- Capture a development-loop baseline when optimizing iteration workflows (`cargo perf dev-loop-baseline --output <path>`).
- Profile before tuning hot paths (CPU and/or memory, depending on symptom).
- Compare against a baseline after changes (`cargo perf compare <name>`).
- Record measured results and decisions in PRs and relevant docs/wiki pages.

## Workspace Performance Surface Map (Benchmark Coverage Targets)

These are the minimum performance surfaces that should be covered by unit tests, integration tests, and benchmarks as the project matures.

| Surface | Crate(s) | Example workloads | Test/benchmark emphasis |
| --- | --- | --- | --- |
| Desktop state transitions and reducers | `crates/desktop_runtime` | window open/close/focus, taskbar updates, launcher actions, reducer batches | unit tests for invariants and boundaries; Criterion microbenches for reducer/event throughput |
| Storage and persistence adapters | `crates/platform_storage`, `crates/platform_host_web` | serialization/deserialization, storage reads/writes, migration paths, cache access | unit + integration tests with realistic payloads; memory profiling for allocation-heavy paths |
| Host contracts and integration boundaries | `crates/platform_host`, `crates/platform_storage`, `crates/platform_host_web` | contract conversions, error mapping, async boundary calls | integration tests for correctness and latency-sensitive boundary behavior |
| App-level reducers and interactions | `crates/apps/*` | editor edits, file navigation, calculator history/tape, terminal transcript growth | unit tests for edge cases; workload benches for sustained interaction sequences |
| Web/WASM startup and route bootstrap | `crates/site`, `crates/desktop_runtime` | deep-link parsing, initial mount, shell bootstrap, app mounting | WASM/browser timing instrumentation and scenario tests under realistic page loads |
| Concurrency and async coordination | any crate using channels/tasks/shared state | task fan-out, synchronization, background persistence loops | stress/integration tests, tail-latency benchmarks, deadlock/race checks |
| I/O boundaries | storage/cache/file APIs | repeated reads/writes, large payloads, error/retry loops | integration tests with fixtures, throughput/latency benches, memory growth tracking |

## Test Coverage Expectations (Correctness + Performance Safety)

### Unit Tests

Required for:

- core reducers/state transitions
- parsers and serializers
- boundary validation and error handling
- invariants around IDs, ordering, and lifecycle state

### Integration Tests

Required for:

- cross-crate behavior and host/storage boundaries
- realistic workloads (batch actions, large payloads, repeated sessions)
- I/O edge cases and retry/failure behavior
- concurrency-sensitive flows (where applicable)

### Benchmarks

Use Criterion (`criterion`) for statistically rigorous benchmarking of critical paths.

Minimum benchmark categories to maintain over time:

- Microbenchmarks: tight hot paths (reducers, parsers, serialization, diffing)
- Scenario benchmarks: user-like multi-step workflows (open app, mutate state, persist, restore)
- Concurrency benchmarks: contention/tail-latency measurements for shared structures or channels
- I/O boundary benchmarks: payload-size sweeps and repeated operation throughput
- WASM execution benchmarks: browser-executed timing marks or automated workload runs where applicable

## Tooling Standard

### Benchmarking

- `cargo bench` for benchmark target execution
- `criterion` for statistically robust measurements, baseline capture, and comparison
- `cargo perf bench` as the standardized workspace entry point
- `cargo perf baseline <name>` / `cargo perf compare <name>` for Criterion baseline workflows
- `cargo perf dev-loop-baseline [--output <path>]` for repeatable local feedback-loop timing snapshots

### CPU Profiling

- `cargo flamegraph` for quick CPU hot-path capture and visualization
- `perf` (`perf record`, `perf report`, `perf stat`) for lower-level Linux profiling and counter analysis
- `cargo perf flamegraph ...` as the standardized flamegraph entry point (auto-writes SVG under `.artifacts/perf/flamegraphs/` unless overridden)

### Memory Profiling

- `heaptrack` (or platform-equivalent tool) for allocation growth and heap hotspot analysis
- `cargo perf heaptrack -- <command...>` for repeatable local heaptrack capture (defaults to `cargo bench --workspace`)

### Compiler Caching

- `sccache` for local compiler artifact reuse across rebuilds
- `cargo perf doctor` reports `sccache` availability plus active `RUSTC_WRAPPER` status
- local setup helper script: `source scripts/dev/setup-sccache.sh`

### Instrumentation (Built-in / Low-overhead)

Use built-in instrumentation where profiling tools are too coarse or unavailable (especially browser/WASM paths):

- `std::time::Instant` timing around scoped operations
- feature-gated counters/timers (`cfg(feature = "perf-instrumentation")`) where persistent probes are needed
- structured logging or tracing spans/events (for example `tracing`) to correlate workload phases
- browser `performance.now()` / User Timing marks for WASM startup and UI workflows (reported through test harnesses or manual traces)

Instrumentation must be removable or feature-gated when it materially impacts code clarity or runtime overhead.

## Baselines and Regression Thresholds

Thresholds below are defaults for local and controlled benchmark reviews. Individual workloads may define tighter thresholds when noise is well-characterized.

### Runtime/Throughput Benchmarks (Criterion)

- Baseline capture: required before tuning critical paths
- Comparison mode: required after changes to benchmarked code
- Suggested interpretation:
  - `< 5%` delta: usually noise or minor change; review context
  - `5-10%` delta: investigate and document cause
  - `> 10%` regression on critical path: treat as blocking unless justified and approved
  - `> 10%` improvement: validate correctness and portability before merging

### Memory Behavior

- Peak RSS / heap growth regression thresholds (scenario-dependent):
  - `>= 10%` increase: investigate
  - `>= 20%` increase on critical path/workload: blocking unless justified
- Allocation count/bytes should be tracked for hot loops and repeated UI actions where feasible.

### Concurrency / Tail Latency

- Evaluate p95/p99 latency for contention-sensitive paths, not only averages.
- Regressions in tail latency should be treated as user-facing risk even if mean latency improves.

### WASM / Browser Execution

- Compare startup and interaction timings in a stable browser/runtime configuration.
- Use repeated runs and medians; avoid single-run comparisons.
- Treat browser/toolchain version changes as baseline invalidation events unless explicitly controlled.

## Controlled Benchmark Environment Guidance (Optional but Recommended)

For regression detection with tighter thresholds, run benchmark suites in a controlled environment (local dedicated machine, lab host, or manually triggered benchmark runner).

Recommended controls:

- fixed machine type/CPU governor and low background load
- fixed Rust toolchain and target triple
- fixed browser version for WASM scenarios
- warmed caches when measuring steady-state throughput (and separately measured cold-start behavior)
- recorded environment metadata (tool versions, OS, commit SHA, benchmark command)

## Artifact Locations and Retention (Local Convention)

- `.artifacts/perf/`: root performance artifacts directory
- `.artifacts/perf/flamegraphs/`: default flamegraph SVG outputs from `cargo perf flamegraph`
- `.artifacts/perf/reports/`: reserved for summarized benchmark/profile reports or exported comparisons
- `target/criterion/`: Criterion benchmark outputs/baselines (tool-managed default location)

## Dependency Hygiene Snapshot

When evaluating compile-time regressions, capture duplicate dependency families and high-churn versions:

```bash
cargo tree -d --workspace
```

Record notable multi-version families in optimization review notes when they materially affect compile time.

## Optimization Decision Documentation Requirements

When optimization changes are proposed, document:

- workload and symptom being optimized
- baseline command(s) and environment summary
- profile evidence (CPU and/or memory)
- measured before/after results
- correctness validation commands run
- tradeoffs (readability, maintainability, portability, complexity)
- follow-up work deferred (if any)

Record the summary in the PR and update relevant rustdoc/wiki/docs pages when behavior, interfaces, or operational guidance changed.

## Standard Command Reference (Performance Workflow)

```bash
cargo perf doctor
cargo perf check
cargo perf bench
cargo perf baseline <name>
cargo perf compare <name>
cargo perf dev-loop-baseline --output .artifacts/perf/reports/dev-loop-baseline.json
cargo perf flamegraph --bench <bench_name>
cargo perf heaptrack -- cargo bench --workspace
```

Linux low-level CPU profiling examples (when `perf` is available):

```bash
perf stat cargo bench --workspace
perf record --call-graph dwarf cargo bench --workspace
perf report
```

## Related Documents

- [`docs/sop/performance-engineering-sop.md`](../sop/performance-engineering-sop.md)
- [`docs/reference/project-command-entrypoints.md`](project-command-entrypoints.md)
- Wiki explanation/how-to/reference pages for performance engineering (see the wiki hub)
