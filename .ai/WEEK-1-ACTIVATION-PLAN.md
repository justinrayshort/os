# WEEK 1 ACTIVATION PLAN: Phase 1 P0 Actions

**Objective**: Establish CI gates, fix critical gaps, unblock scaling work
**Duration**: 5 working days (35 hours total)
**Team Capacity**: Assumes ~7 hours focused work per day
**Status**: READY FOR EXECUTION

---

## EXECUTIVE SUMMARY

This plan translates the strategic roadmap into executable daily tasks. Each day has:
- **Morning Goal**: Specific, measurable outcome
- **Implementation Steps**: Concrete file edits and commands
- **Verification**: How to confirm success
- **Success Metric**: What passes/fails to determine completion
- **Blockers**: Known risks and mitigation

All tasks reference exact file paths and line numbers for rapid navigation.

---

## DAILY BREAKDOWN

### **DAY 1: Dependency & Profile Foundations (2.5 hours)**

**Morning Goal**: Enable cargo-deny CI gate + optimize release profile
**Timeboxed**: 2.5 hours

#### Task 1.1: Set Up cargo-deny Circular Dependency Detection (1 hour)

**What**:  Add `cargo deny check` as pre-commit/CI gate to prevent circular dependencies from being introduced.

**Implementation**:

1. **Create `deny.toml` in workspace root**:
   ```bash
   # Run from /Users/justinshort/os
   touch deny.toml
   ```

2. **Populate with circular dependency checks**:
   - Add check for circular dependencies
   - Configure advisory/bans/sources sections
   - Reference: https://embarkstudios.github.io/cargo-deny/

3. **Verify locally**:
   ```bash
   cargo deny check
   ```

4. **Add to CI workflow** (reference implementation):
   - Edit: `.github/workflows/ci.yml` or equivalent
   - Add step: `cargo deny check`
   - Should block PRs if circular deps introduced

**Success Metric**:
- ✓ `cargo deny check` executes without panicking
- ✓ Reports 0 circular dependencies
- ✓ CI workflow includes denial check

**File References**:
- New: `/Users/justinshort/os/deny.toml` (50-100 lines)
- Modify: `.github/workflows/` (add step)

**Blockers**:
- Unknown: Does project use GitHub Actions or different CI?
- Mitigation: Check `.github/workflows/` directory for existing workflow patterns

---

#### Task 1.2: Optimize Release Profile (1.5 hours)

**What**: Enable LTO, codegen-units, and stripping in release profile to achieve 5-10% binary size reduction.

**Implementation**:

1. **Read current Cargo.toml**:
   ```bash
   head -50 /Users/justinshort/os/Cargo.toml
   ```

2. **Add [profile.release] section to workspace root**:
   - Location: After `[workspace]` section in Cargo.toml
   - Settings:
     ```toml
     [profile.release]
     opt-level = 3
     lto = "thin"           # Enables thin LTO (faster than full, good compression)
     codegen-units = 256    # Default; keep for faster release builds (vs. 1 for max optimization)
     strip = true           # Strip symbols from release binary
     ```

3. **Verify compilation**:
   ```bash
   cargo build --release
   ```

4. **Measure binary size before/after**:
   ```bash
   # Find release binary (likely in target/release/)
   ls -lh target/release/ | grep -E '(shell|app|binary-name)'
   ```

**Success Metric**:
- ✓ `cargo build --release` completes without errors
- ✓ Release binary is smaller than before (target: 5-10% reduction)
- ✓ All crates linked successfully with LTO enabled

**File References**:
- Modify: `/Users/justinshort/os/Cargo.toml` (add ~5 lines)

**Verification**:
```bash
# Confirm profile settings are respected
cargo rustc --release -- --print=codegen-units
```

---

### **DAY 2: Documentation & Cleanup Gates (4 hours)**

**Morning Goal**: Enforce documentation freshness in CI + implement artifact cleanup command
**Timeboxed**: 4 hours

#### Task 2.1: Implement Doc Freshness CI Gate (2 hours)

**What**: Add CI gate that blocks PRs if any documentation has not been reviewed in > 90 days.

**Implementation**:

1. **Identify doc validation location**:
   - Check: `xtask/src/commands/docs.rs` (from evaluation, docs command family exists)
   - Look for: frontmatter validation, audit logic

2. **Enhance `cargo xtask docs frontmatter` validation**:
   - Currently: advisory last_reviewed tracking
   - Target: CI-blocking gate (90d threshold)
   - Modify logic: If any doc `last_reviewed` > 90 days ago, exit with error code 1

3. **Update xtask docs command**:
   - Add subcommand or flag: `cargo xtask docs audit --enforce-freshness`
   - Output: List of stale docs and owners
   - Exit code: 1 if violations found (CI will block)

4. **Add to CI workflow**:
   - Step: `cargo xtask docs audit --enforce-freshness`
   - Should run on every PR
   - Fails if any doc is stale

**Success Metric**:
- ✓ `cargo xtask docs audit --enforce-freshness` executes
- ✓ Reports any stale docs with owner assignment
- ✓ CI fails if stale docs detected

**File References**:
- Modify: `xtask/src/commands/docs.rs` (estimated 50-100 line addition)
- Modify: `.github/workflows/` (add CI step)

**Verification**:
```bash
# Test locally
cargo xtask docs audit --enforce-freshness
# Should pass or fail with clear output
```

---

#### Task 2.2: Implement Artifact Cleanup Command (2 hours)

**What**: Create `cargo xtask cleanup` command to manage unbounded `.artifacts/` growth.

**Implementation**:

1. **Create new cleanup command module**:
   - Location: `xtask/src/commands/cleanup.rs`
   - Repository reference: Similar structure to existing commands (cache.rs, dev.rs)

2. **Define cleanup policy** (from roadmap):
   - Dev artifacts: 7 days
   - Release artifacts: 30 days
   - Baselines: 90 days
   - sccache: 30GB limit

3. **Implement deletion logic**:
   ```rust
   // Pseudocode
   fn cleanup_artifacts(ctx: &CommandContext) -> Result<()> {
       let now = std::time::SystemTime::now();

       // Remove dev artifacts older than 7 days
       for artifact in ctx.artifacts().dev_runs() {
           if artifact.age() > Duration::days(7) {
               fs::remove_dir_all(&artifact.path())?;
           }
       }

       // Similar for release (30d) and baselines (90d)
       // Report stats: "Removed 250MB from dev artifacts, 100MB from baselines"
   }
   ```

4. **Add to CLI dispatcher**:
   - Register in `xtask/src/cli/mod.rs`
   - Subcommand: `cleanup [--dry-run] [--policy dev|release|baseline|all]`

5. **Integrate with doctor command**:
   - Run cleanup suggestion if `.artifacts/` > 5GB

**Success Metric**:
- ✓ `cargo xtask cleanup --dry-run` reports what would be deleted
- ✓ `cargo xtask cleanup` executes without panicking
- ✓ `.artifacts/` size reduced per policy
- ✓ Respects `.keep` markers for important baselines

**File References**:
- New: `xtask/src/commands/cleanup.rs` (150-200 lines)
- Modify: `xtask/src/commands/mod.rs` (register module)
- Modify: `xtask/src/cli/mod.rs` (add CLI dispatch)

**Verification**:
```bash
# Test dry-run
cargo xtask cleanup --dry-run
# Should list files without deleting
```

---

### **DAY 3: Compile-Time SLO Framework (2.5 hours)**

**Morning Goal**: Define and enforce compile-time performance thresholds in CI
**Timeboxed**: 2.5 hours

#### Task 3.1: Define Compile-Time SLOs (1 hour)

**What**: Establish performance baselines and SLO thresholds for compilation speed.

**Implementation**:

1. **Create `tools/automation/compile-slos.toml`**:
   ```toml
   [slos]
   # Baseline thresholds (from evaluation: check 48.9s, test 45.9s)
   check_threshold_seconds = 55      # Allow 6s regression buffer
   test_compile_threshold_seconds = 50
   release_threshold_seconds = 600   # 10 minutes for clean release build

   [tolerances]
   maximum_regression_percent = 5    # Fail if > 5% slower than baseline

   [tracking]
   baseline_date = "2026-02-28"
   baseline_check_seconds = 48.9
   baseline_test_seconds = 45.9
   ```

2. **Document policy in README or PERFORMANCE.md**:
   - Why these thresholds matter
   - How to handle legitimate slowdowns
   - How to update baselines quarterly

**Success Metric**:
- ✓ SLO file created and readable
- ✓ All thresholds documented with rationale
- ✓ Visible in xtask perf or similar commands

**File References**:
- New: `tools/automation/compile-slos.toml` (20 lines)
- Modify: `docs/reference/performance-engineering.md` (document thresholds)

---

#### Task 3.2: Integrate SLO Checks into Verify Commands (1.5 hours)

**What**: Add `cargo verify` step that measures compile time and fails if > threshold.

**Implementation**:

1. **Locate verify command**:
   - Reference: `xtask/src/commands/verify.rs` (from evaluation)
   - Current profiles: dev, ci-fast, ci-full, release

2. **Add timing measurement**:
   - Before compilation: record start time
   - Run: `cargo check --workspace`
   - After compilation: record end time
   - Compare: actual vs. threshold from compile-slos.toml

3. **Add SLO check logic**:
   ```rust
   // Pseudocode
   let compile_time_secs = measure_compilation(&ctx)?;
   let threshold = load_slos(&ctx)?;

   if compile_time_secs > threshold.check_threshold_seconds {
       eprintln!("⚠️  Compile time regression: {}s > {}s threshold",
                 compile_time_secs, threshold.check_threshold_seconds);
       return Err(VerifyError::CompileSloBreach);
   }
   ```

4. **Update verify output**:
   - Show: "✓ Compile time: 49.2s (within 55s SLO)"
   - Or: "✗ Compile time: 58.1s (EXCEEDS 55s SLO by 3.1s)"

**Success Metric**:
- ✓ `cargo verify` reports compile time
- ✓ Passes if within threshold
- ✓ Fails with clear message if exceeds threshold
- ✓ Exits with error code 1 on breach (CI can block)

**File References**:
- Modify: `xtask/src/commands/verify.rs` (add ~30 lines for timing + comparison)

**Verification**:
```bash
# Run verify and confirm SLO output
cargo verify --profile ci-full
# Should show "Compile time: 49.2s (within 55s SLO)"
```

---

### **DAY 4: E2E Refactoring Phase 1 (6 hours)**

**Morning Goal**: Extract e2e.rs types and config into separate modules (first step of monolith modularization)
**Timeboxed**: 6 hours

#### Task 4.1: Extract e2e Types Module (2 hours)

**What**: Move all type definitions from e2e.rs into dedicated types.rs module.

**Implementation**:

1. **Read e2e.rs to identify types** (from evaluation: 2,338 lines):
   - Reference: `xtask/src/commands/e2e.rs`
   - Identify all `struct`, `enum`, type aliases used in e2e workflow

2. **Create `xtask/src/commands/e2e/types.rs`**:
   - Location: New file in e2e subdirectory
   - Content: All type definitions
   - Examples (estimate): PlaywrightConfig, TauriConfig, BaselineManifest, RunOptions, etc.

3. **Update e2e.rs to re-export types**:
   ```rust
   // At top of xtask/src/commands/e2e.rs
   mod types;
   pub use types::*;

   // Rest of e2e logic uses types from this module
   ```

4. **Organize e2e directory structure**:
   ```
   xtask/src/commands/e2e/
   ├── mod.rs           (now: dispatcher + public API)
   ├── types.rs         (new: all type definitions)
   ├── config.rs        (next: config loading - Phase 2)
   ├── harness.rs       (future: Playwright/Tauri logic)
   └── manifest.rs      (future: baseline management)
   ```

**Success Metric**:
- ✓ `cargo check --package xtask` passes
- ✓ All types are in types.rs
- ✓ e2e.rs no longer defines types (only functions)
- ✓ Line count: e2e.rs reduced from 2,338 → ~1,200 (progress)

**File References**:
- Modify: `xtask/src/commands/e2e.rs` (split into directory structure)
- New: `xtask/src/commands/e2e/mod.rs` (dispatcher)
- New: `xtask/src/commands/e2e/types.rs` (type definitions)

**Verification**:
```bash
# Check line count reduction
wc -l xtask/src/commands/e2e.rs     # Should drop from 2,338
wc -l xtask/src/commands/e2e/mod.rs  # New file
wc -l xtask/src/commands/e2e/types.rs

# Verify compilation
cargo check --package xtask
cargo test --package xtask
```

---

#### Task 4.2: Extract e2e Config Module (2 hours)

**What**: Extract profile configuration loading and parsing into config.rs.

**Implementation**:

1. **Identify configuration logic in e2e.rs**:
   - Profile detection (local-dev, ci-headless, cross-browser)
   - Option parsing for --profile, --filter, --update-baseline
   - Environment variable handling

2. **Create `xtask/src/commands/e2e/config.rs`**:
   ```rust
   // e2e/config.rs
   pub struct E2eConfig {
       pub profile: E2eProfile,
       pub filters: Vec<String>,
       pub update_baseline: bool,
       pub retry_count: u32,
   }

   impl E2eConfig {
       pub fn from_options(opts: &E2eOptions) -> Result<Self> {
           // Load from opts, environment, defaults
       }
   }
   ```

3. **Move all parse_* functions**:
   - `parse_run_options()` → config.rs
   - Profile loading → config.rs
   - Filter compilation → config.rs

4. **Update e2e/mod.rs**:
   - Import config module
   - Change function signature: `pub fn e2e_run(opts: E2eOptions) -> Result<()>`
   - Inside: `let cfg = E2eConfig::from_options(&opts)?;`

**Success Metric**:
- ✓ `cargo check --package xtask` passes
- ✓ Config loading is isolated in config.rs
- ✓ e2e/mod.rs is now dispatcher + execution (no parsing logic)
- ✓ e2e size reduced further: 2,338 → ~1,000 LOC

**File References**:
- New: `xtask/src/commands/e2e/config.rs` (150-200 lines)
- Modify: `xtask/src/commands/e2e/mod.rs` (integrate config)

**Verification**:
```bash
# Verify compilation and structure
cargo check --package xtask

# Run e2e command to confirm it still works
cargo xtask e2e --help
```

---

#### Task 4.3: Update Command Registration (1 hour)

**What**: Ensure e2e command still dispatches correctly after directory restructuring.

**Implementation**:

1. **Update `xtask/src/commands/mod.rs`**:
   - Find: reference to `mod e2e;`
   - Should still work (Rust will find `e2e/mod.rs`)
   - Verify: no other imports need updating

2. **Test e2e command**:
   ```bash
   cargo xtask e2e --help
   cargo xtask e2e ci-headless
   ```

**Success Metric**:
- ✓ `cargo xtask e2e` works as before
- ✓ All profiles accessible (--profile local-dev, ci-headless, etc.)
- ✓ Help text accurate

---

### **DAY 5: Unit Test Scaffolding (12+ hours - Multi-day Task)**

**Morning Goal**: Begin reducer unit test suite implementation (Phase 1 foundation)
**Timeboxed**: Full day (8 hours) + overflow to next week

#### Task 5.1: Set Up Test Structure (1 hour)

**What**: Create test module organization for reducer tests.

**Implementation**:

1. **Locate reducer**:
   - Reference: `crates/desktop_runtime/src/reducer.rs` (from evaluation)
   - Current: 19 tests for 43 DesktopAction variants

2. **Create test submodule in reducer.rs**:
   ```rust
   // At end of reducer.rs
   #[cfg(test)]
   mod tests {
       use super::*;

       mod action_tests;
       mod invariant_tests;
       mod edge_cases;
   }
   ```

3. **Create test directory structure**:
   ```
   crates/desktop_runtime/src/
   ├── reducer.rs
   └── reducer/
       ├── mod.rs        (tests:: module)
       ├── action_tests.rs   (43 action variants)
       ├── invariant_tests.rs (window ordering, z-index, focus)
       └── fixtures.rs   (shared test data)
   ```

**Success Metric**:
- ✓ Test module compiles
- ✓ Structure supports 120+ tests without confusion
- ✓ Clear organization: one test type per file

---

#### Task 5.2: Implement Basic Fixtures (2 hours)

**What**: Create reusable test fixtures for common reducer states.

**Implementation**:

1. **Create `crates/desktop_runtime/src/reducer/fixtures.rs`**:
   ```rust
   pub fn default_state() -> DesktopState { ... }
   pub fn state_with_apps(apps: &[&str]) -> DesktopState { ... }
   pub fn state_with_windows(count: usize) -> DesktopState { ... }
   pub fn state_with_focused_window(app_id: &str) -> DesktopState { ... }
   ```

2. **Establish naming convention**:
   - Test name: `test_action_<variant>_<scenario>`
   - Example: `test_action_open_app_creates_window`
   - Example: `test_action_focus_window_changes_z_index`

3. **Draft test template**:
   ```rust
   #[test]
   fn test_action_open_app_creates_window() {
       let mut state = default_state();
       let action = DesktopAction::OpenApp {
           id: "terminal".into(),
           args: vec![]
       };

       let result = reduce(&state, action);

       assert!(result.is_ok());
       // More assertions...
   }
   ```

**Success Metric**:
- ✓ Fixtures compile and are usable
- ✓ Template test executes
- ✓ Clear pattern for remaining tests

---

#### Task 5.3: Implement OpenApp Action Tests (2 hours)

**What**: Write comprehensive tests for DesktopAction::OpenApp variant.

**Implementation**:

1. **Create test cases for OpenApp**:
   - Happy path: app opens, window created, z-index set
   - Edge case: app already open (duplicate handling)
   - Edge case: app_id empty string
   - Edge case: invalid app_id
   - Invariant: z-index of new window is highest
   - Invariant: app appears in app registry after open

2. **Each test follows pattern**:
   - Setup initial state
   - Create action
   - Call reduce()
   - Assert result and state

3. **Target**: 5-8 tests for OpenApp

**Success Metric**:
- ✓ 5-8 tests for OpenApp
- ✓ All pass with `cargo test --lib`
- ✓ Edge cases documented in test names

---

#### Task 5.4: Implement FocusWindow Action Tests (2 hours)

**What**: Comprehensive tests for DesktopAction::FocusWindow (z-index, focus state).

**Implementation**:

1. **Test cases for FocusWindow**:
   - Happy path: window marked focused, z-index moved to top
   - Edge case: focus non-existent window (error handling)
   - Edge case: refocus already-focused window (idempotent)
   - Invariant: z-order preserved (other windows unchanged)
   - Invariant: only one window can be focused at a time

2. **Target**: 6-8 tests for FocusWindow

---

#### Task 5.5: Implement State Invariant Tests (3 hours)

**What**: Test cross-cutting concerns that must hold for ALL state transitions.

**Implementation**:

1. **Create `crates/desktop_runtime/src/reducer/invariant_tests.rs`**:
   ```rust
   // Template: for each action variant, verify invariants hold

   #[test]
   fn invariant_z_index_unique_per_window() {
       // For all states, z-indices must be unique
   }

   #[test]
   fn invariant_max_one_focused_window() {
       // Never two focused windows
   }

   #[test]
   fn invariant_app_registry_consistent() {
       // Apps in windows match apps in registry
   }
   ```

2. **Invariants to test** (from evaluation):
   - Z-index uniqueness and ordering
   - At most one focused window
   - App registry consistency
   - No orphaned windows
   - Focus always on existing window

3. **Target**: 8-12 invariant tests

---

#### Task 5.6: Status & Next Steps (0.5 hours)

**What**: Assess progress against target (50+ tests, > 60% coverage).

**Success Metric (End of Day 5)**:
- ✓ Scaffolding complete (directory structure, fixtures)
- ✓ ~15-20 tests implemented (OpenApp, FocusWindow, invariants)
- ✓ All tests passing
- ✓ Estimated coverage: 35-40% (progress toward 60% goal)

**Remaining Work** (Overflow to Week 2):
- Implement tests for remaining 41 DesktopAction variants
- Target: 30-35 more tests needed for 60%+ coverage
- Estimated effort: 8-10 hours (Days 6-7)

**File References**:
- New: `crates/desktop_runtime/src/reducer/` directory (3-4 files)
- Modify: `crates/desktop_runtime/src/reducer.rs` (add module declaration)

**Verification**:
```bash
# Run tests
cargo test --lib reducer --

# Check coverage (if tarpaulin available)
cargo tarpaulin --package desktop_runtime --lib
```

---

## WEEK 1 COMPLETION SUMMARY

### Deliverables (Expected)
- ✓ cargo-deny CI gate operational (0 circular deps enforced)
- ✓ Release profile optimized (LTO enabled, strip configured)
- ✓ Doc freshness gate functional (90-day enforcement)
- ✓ Artifact cleanup command available
- ✓ Compile-time SLOs defined and integrated into verify
- ✓ e2e.rs types/config modules extracted (~35% size reduction in progress)
- ✓ Reducer test suite scaffolding + initial ~20 tests (35-40% coverage achieved)

### Metrics Progress
| Metric | Baseline | End of Week 1 | Target | Progress |
|--------|----------|---------------|--------|----------|
| Circular deps | 0 | 0 (enforced) | 0 | ✓ Complete |
| Compile SLO enforcement | None | Active | Enforced | ✓ Complete |
| Reducer test coverage | 19 tests (49%) | ~35 tests (40%) | 50+ (60%) | 70% progress |
| e2e.rs size | 2,338 LOC | ~1,800 LOC | 150 LOC | 23% progress |
| Doc staleness | 0 stale | 0 (enforced) | 0 | ✓ Complete |

### Go/No-Go Gate (End of Week 2)
**Phase 1 completion requires**:
- [ ] All P0 actions complete ✓ (on track)
- [ ] CI gates blocking regressions ✓ (circular deps, SLOs, doc freshness)
- [ ] No new failures introduced (verify with `cargo test --workspace`)

---

## CONTINUATION NOTES FOR WEEK 2

**Week 2 picks up with**:
1. **Finish Reducer Tests** (8-10 hours): Complete 50+ test suite, achieve 60%+ coverage
2. **Integration Test Scaffolding** (8 hours): Create AppTestHarness abstraction
3. **e2e.rs Continued** (6 hours): Extract harness module (partial completion)
4. **P1 Actions Start**: File I/O helpers, sccache metrics, E2E determinism baseline

**Success Gate**: By end of Week 2, all P0 actions must be complete and CI gates operational.

---

## QUICK REFERENCE: FILE LOCATIONS

| Task | File | Action |
|------|------|--------|
| cargo-deny | `deny.toml` (new) | Circular dep detection |
| Release profile | `Cargo.toml` | Add [profile.release] |
| Doc gate | `xtask/src/commands/docs.rs` | Enhance audit command |
| Artifact cleanup | `xtask/src/commands/cleanup.rs` (new) | New command |
| SLOs | `tools/automation/compile-slos.toml` (new) | Define thresholds |
| Verify SLOs | `xtask/src/commands/verify.rs` | Integrate checks |
| e2e refactor | `xtask/src/commands/e2e/` (directory) | Split module |
| Reducer tests | `crates/desktop_runtime/src/reducer/` (new) | Test suite |

---

**Plan Status**: READY FOR EXECUTION
**Created**: Generated from SYSTEMS-EVALUATION-ROADMAP.md
**Authority**: Derives from Phase 1 P0 actions (critical, blocking)
**Approval Required**: None (execution can begin immediately on Day 1)

