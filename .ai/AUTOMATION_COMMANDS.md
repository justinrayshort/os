# Automation Commands Reference

**Version:** 1.0  
**Last Updated:** 2026-03-01  
**Purpose:** Catalog of Cargo aliases and xtask commands available for local development and CI.

All commands listed here are defined in `.cargo/config.toml` and `xtask/` and are the primary interface for build, test, docs, and performance workflows.

## Quick Start (Most Common)

```bash
cargo verify-fast        # Fast verification (format, tests, docs build)
cargo verify             # Full verification (all profiles, clippy, docs)
cargo test --workspace   # Run all tests
cargo fmt --all          # Format code
cargo doc --workspace --no-deps  # Build rustdoc
```

## Verification Workflows

### cargo verify-fast

**Purpose:** Quick local verification (primary dev loop)

**Runs:**
1. Format check (cargo fmt --all -- --check)
2. Workspace tests (cargo test --workspace)
3. Docs build (cargo doc --workspace --no-deps)
4. Docs contract validation (cargo xtask docs all)

**Duration:** ~1-2 minutes  
**Exit Code:** 0 on success, non-zero on failure

**When to use:** Before committing, after code changes, during development iteration

```bash
cargo verify-fast
```

### cargo verify

**Purpose:** Full verification (all profiles and linting)

**Runs:**
1. All cargo verify-fast checks
2. Clippy linting (cargo clippy --workspace --all-targets)
3. Verification profiles (as defined in tools/automation/verify_profiles.toml)
4. Full docs audit (if needed)

**Duration:** ~5-10 minutes  
**Exit Code:** 0 on success

**When to use:** Before pushing, in CI pipeline, before requesting review

```bash
cargo verify
```

## Testing Commands

### cargo test --workspace

**Purpose:** Run all unit and integration tests

**Runs:** All tests in all crates

```bash
cargo test --workspace
```

### cargo test --workspace --doc

**Purpose:** Run doctests (tests in rustdoc examples)

**Runs:** All code examples in rustdoc comments

**When to use:** After updating rustdoc examples

```bash
cargo test --workspace --doc
```

### cargo test -p CRATE_NAME

**Purpose:** Run tests for specific crate

**Example:**
```bash
cargo test -p desktop_runtime
cargo test -p system_ui
```

## Code Quality Commands

### cargo fmt --all

**Purpose:** Format all code per rustfmt defaults

```bash
cargo fmt --all
```

### cargo fmt --all -- --check

**Purpose:** Check formatting without modifying (CI mode)

```bash
cargo fmt --all -- --check
```

### cargo clippy --workspace --all-targets

**Purpose:** Lint checks (warnings, anti-patterns, style)

```bash
cargo clippy --workspace --all-targets
```

## Documentation Commands

### cargo doc --workspace --no-deps

**Purpose:** Build rustdoc for entire workspace

**Output:** Generated HTML docs in target/doc/

**Review:** Open target/doc/index.html in browser

```bash
cargo doc --workspace --no-deps
```

### cargo docs-check

**Alias for:** `cargo xtask docs all`

**Purpose:** Validate all documentation contracts

**Checks:**
- Frontmatter (required fields present)
- SOP structure (if applicable)
- Wiki instructional template (if applicable)
- OpenAPI specs (if present)
- Mermaid diagrams (if present)
- Broken internal links
- Docs audit report generation

```bash
cargo docs-check
# or
cargo xtask docs all
```

### cargo xtask docs structure

**Purpose:** Validate docs folder structure and category mapping

```bash
cargo xtask docs structure
```

### cargo xtask docs frontmatter

**Purpose:** Validate frontmatter in all docs/*.md files

Required fields checked: title, category, owner, status, last_reviewed, audience, invariants

```bash
cargo xtask docs frontmatter
```

### cargo xtask docs sop

**Purpose:** Validate SOP document structure (required section headings)

```bash
cargo xtask docs sop
```

### cargo xtask docs wiki

**Purpose:** Validate external wiki structure and instructional templates

**Requires:** Wiki cloned locally (`cargo wiki clone`)

```bash
cargo xtask docs wiki
```

### cargo xtask docs links

**Purpose:** Detect broken internal references in docs

```bash
cargo xtask docs links
```

### cargo xtask docs storage-boundary

**Purpose:** Enforce typed app-state boundary rules (prevents direct envelope-load usage in app/runtime crates)

```bash
cargo xtask docs storage-boundary
```

### cargo xtask docs ui-conformance

**Purpose:** Audit UI token/primitive hygiene and design-system conformance

**Checks:**
- Shared primitive token usage (no placeholder values)
- Icon centralization (use system_ui IconName only)
- No raw interactive markup in app/runtime surfaces
- No direct shared-primitive data-ui-kind composition in app/runtime
- No new app/runtime-local layout-only class contracts

```bash
cargo xtask docs ui-conformance
```

### cargo xtask docs audit-report --output PATH

**Purpose:** Generate audit artifact for documentation governance tracking

**Output:** JSON report with contract validation status, review freshness, coverage

```bash
cargo xtask docs audit-report --output .artifacts/docs-audit.json
```

## Wiki Commands

### cargo wiki clone

**Purpose:** Clone external GitHub Wiki repository (canonical docs hub)

**Downloads:** Wiki content to local wiki/ directory for validation

```bash
cargo wiki clone
```

### cargo wiki

**Alias for:** `cargo xtask wiki`

**Purpose:** Manage wiki integration (clone, validate, etc.)

```bash
cargo wiki clone
```

## Performance & Benchmarking

### cargo perf check

**Purpose:** Validate performance measurement setup (compile benchmarks, run tests)

```bash
cargo perf check
```

### cargo perf bench

**Purpose:** Run all benchmarks and collect baseline metrics

```bash
cargo perf bench
```

### cargo perf baseline LOCAL_NAME

**Purpose:** Save benchmark baseline with given name

```bash
cargo perf baseline local-main
```

### cargo perf compare BASELINE_NAME

**Purpose:** Compare current code against saved baseline

```bash
cargo perf compare local-main
```

### cargo perf dev-loop-baseline --output PATH

**Purpose:** Record current dev-loop performance baseline

```bash
cargo perf dev-loop-baseline --output .artifacts/perf/reports/dev-loop-baseline.json
```

### cargo perf flamegraph --bench NAME

**Purpose:** Generate flamegraph for specific benchmark

**Requires:** Optional flamegraph tools

```bash
cargo perf flamegraph --bench desktop_runtime_bench
```

### cargo perf heaptrack -- COMMAND

**Purpose:** Profile memory usage with heaptrack

**Requires:** Optional heaptrack binary

```bash
cargo perf heaptrack -- cargo bench
```

## E2E Testing (Browser UI Tests)

### cargo e2e doctor

**Purpose:** Validate E2E test environment and dependencies

```bash
cargo e2e doctor
```

### cargo e2e list

**Purpose:** List available E2E test scenarios

```bash
cargo e2e list
```

### cargo e2e run --profile PROFILE --dry-run

**Purpose:** Dry-run E2E tests (compile without execution)

**Profiles:** local-dev, ci-headless, ci-full

```bash
cargo e2e run --profile local-dev --dry-run
```

### cargo e2e run --profile PROFILE --scenario SCENARIO --slice SLICE --no-diff

**Purpose:** Run specific E2E test scenario

**Example:**
```bash
cargo e2e run --profile local-dev --scenario ui.shell.layout-baseline --slice shell.soft-neumorphic.default --no-diff
```

### cargo e2e inspect --run RUN_ID

**Purpose:** Inspect results of specific E2E run

```bash
cargo e2e inspect --run abc123def456
```

### cargo e2e promote --profile PROFILE --scenario SCENARIO --slice SLICE --source-run RUN_ID

**Purpose:** Promote E2E results to baseline

```bash
cargo e2e promote --profile local-dev --scenario ui.shell.layout-baseline --slice shell.soft-neumorphic.default --source-run abc123
```

## Development Server Commands

### cargo dev

**Alias for:** `cargo xtask dev`

**Purpose:** Start development server

```bash
cargo dev
```

### cargo dev serve

**Purpose:** Start local development server (web)

```bash
cargo dev serve
```

### cargo dev stop

**Purpose:** Stop running development server

```bash
cargo dev stop
```

### cargo dev status

**Purpose:** Show status of development server

```bash
cargo dev status
```

## Web Build Commands

### cargo setup-web

**Purpose:** Initialize web dependencies and configuration

```bash
cargo setup-web
```

### cargo web-check

**Purpose:** Validate web build configuration

```bash
cargo web-check
```

### cargo web-build

**Purpose:** Build web assets (WASM, JavaScript)

```bash
cargo web-build
```

## Desktop Build Commands

### cargo tauri-dev

**Purpose:** Run Tauri development mode (native desktop app)

```bash
cargo tauri-dev
```

### cargo tauri-build

**Purpose:** Build production Tauri application

```bash
cargo tauri-build
```

## Utility Commands

### cargo doctor

**Purpose:** Run diagnostic checks on development environment

**Checks:**
- Rust toolchain version
- Dependencies and compilation status
- Build cache (sccache) status
- Platform-specific requirements

```bash
cargo doctor
```

### cargo check-all

**Alias for:** `cargo check --workspace`

**Purpose:** Check compilation for all crates

```bash
cargo check-all
```

### cargo test-all

**Alias for:** `cargo test --workspace`

**Purpose:** Run all workspace tests

```bash
cargo test-all
```

### cargo flow

**Purpose:** Run recommended development workflow

Typically: format → test → clippy → docs

```bash
cargo flow
```

## Make Compatibility Wrappers

For cross-platform compatibility, thin make wrappers exist:

```bash
make verify-fast         # calls cargo verify-fast
make verify              # calls cargo verify
make wiki-init           # calls cargo wiki clone
make rustdoc-check       # calls cargo doc --workspace --no-deps
make docs-check          # calls cargo xtask docs all
make proto-serve         # calls cargo dev serve
make proto-stop          # calls cargo dev stop
make proto-status        # calls cargo dev status
```

## Configuration Reference

| Setting | File | Purpose |
|---|---|---|
| Cargo aliases | .cargo/config.toml | Defines cargo-* command shortcuts |
| Verify profiles | tools/automation/verify_profiles.toml | Defines verification matrices (features, profiles) |
| Wiki config | tools/docs/wiki.toml | External wiki repository path and structure |
| Docs contracts | tools/docs/doc_contracts.json | Schema for docs validation |
| E2E profiles | tools/e2e/playwright.config.ts | E2E test execution profiles |

## Tips & Troubleshooting

### Speeding Up Builds

Enable incremental compilation and sccache:

```bash
source scripts/dev/setup-sccache.sh
```

### Cleaning Build Artifacts

```bash
cargo clean
rm -rf .artifacts  # Remove test/perf artifacts (keep .artifacts/ for CI)
```

### Viewing Generated Docs

```bash
cargo doc --workspace --no-deps --open
```

### Running Single Crate Tests

```bash
cargo test -p desktop_runtime -- --nocapture
```

### Debugging Doctests

```bash
cargo test --workspace --doc -- --nocapture
```

All commands should be run from the repository root (/Users/justinshort/os/).
