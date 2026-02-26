# AGENTS.md

This repository is maintained with help from automated agents. Use this file as the repo-specific operating guide.

## 1) Project Scope

- Rust workspace with multiple crates, including:
  - `crates/desktop_runtime`
  - `crates/platform_storage`
  - `crates/site`
  - `crates/apps/*`
  - `xtask`
- Documentation system split across:
  - Rust source comments (`//!`, `///`) -> generated `rustdoc` API reference
  - GitHub Wiki repository as submodule under `wiki/` (tutorials, how-to guides, explanations)
  - Repo-native Markdown under `docs/` for contracts, SOPs, ADRs, and docs tooling reference
  - Validation/audit CLI implemented in Rust via `cargo xtask docs` (`xtask/src/docs.rs`)

## 2) Operating Rules

- Make minimal, reviewable changes that match existing patterns.
- If behavior, API shape, architecture, or procedures change, update docs in the same change.
- Preserve documentation contracts enforced by `tools/docs/doc_contracts.json`.
- Do not weaken validation rules or local verification workflows unless explicitly requested.
- Avoid destructive git commands unless explicitly requested.

## 3) Documentation Contracts (Required)

The docs validator enforces:

- Frontmatter required fields on docs pages:
  - `title`
  - `category`
  - `owner`
  - `status`
  - `last_reviewed`
  - `audience`
  - `invariants`
- Allowed categories:
  - `tutorial`, `how-to`, `reference`, `explanation`, `adr`, `sop`
- Folder/category mapping (Diataxis) under `docs/` must remain consistent.
- SOP docs must include the required SOP headings (validated by `sop` check).
- Review freshness threshold is tracked (currently 180 days) in audit reporting.

## 4) Local Verification Workflows (Current)

### 4.1 Documentation Verification (Local Rust Toolchain)

Primary entry points:

- `cargo xtask docs all`
- `cargo doc --workspace --no-deps`
- `cargo test --workspace --doc`
- `cargo verify-fast`
- `cargo verify`

Stages (local verification order):

1. Wiki submodule validation (`cargo xtask docs wiki`)
2. Docs contract validation (`structure`, `frontmatter`, `sop`)
3. OpenAPI validation (`cargo xtask docs openapi`)
4. Mermaid validation (`cargo xtask docs mermaid`)
5. Broken internal reference detection (`cargo xtask docs links`)
6. Rustdoc build (`cargo doc --workspace --no-deps`, `RUSTDOCFLAGS=-D warnings`)
7. Rustdoc doctests (`cargo test --workspace --doc`)
8. Audit artifact generation (`cargo xtask docs audit-report --output ...`) when needed

### 4.2 Quarterly Documentation Audit (Manual / Local)

Behavior:

- Run locally on a quarterly cadence (or before governance reviews)
- Validates wiki submodule structure (via `audit-report`)
- Generates `.artifacts/docs-audit.json` via `audit-report`
- Fails locally if audit validation fails
- Preserve/share the audit artifact through your normal review process (no hosted CI dependency)

## 5) Local Commands

### 5.1 Docs Tooling Setup (Rust-only)

```bash
cargo build -p xtask
```

### 5.2 Docs Validation (fast path)

Run the standard local docs validation entry point:

```bash
git submodule update --init --recursive
cargo xtask docs all
cargo doc --workspace --no-deps
cargo test --workspace --doc
```

### 5.3 Docs Commands (explicit)

```bash
git submodule update --init --recursive
cargo xtask docs structure
cargo xtask docs wiki
cargo xtask docs frontmatter
cargo xtask docs sop
cargo xtask docs openapi
cargo xtask docs mermaid
cargo xtask docs links
cargo doc --workspace --no-deps
cargo test --workspace --doc
```

### 5.4 Audit Report Command

```bash
cargo xtask docs audit-report --output .artifacts/docs-audit.json
```

### 5.5 Rust Workspace Commands

Prefer direct Cargo commands for clarity (there is no `package.json` script wrapper in this repo). A root `Makefile` exists for convenience and mirrors common verification/docs/prototype commands:

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
```

Common convenience wrappers (delegating to Cargo aliases / `xtask` docs commands):

```bash
make verify-fast
make verify
make wiki-init
make rustdoc-check
make docs-check
make docs-audit
make proto-check
make proto-build
make proto-build-dev
make proto-serve
make proto-start
make proto-stop
make proto-status
make proto-restart
```

## 6) Change Workflows for Agents

### 6.1 Docs-only changes

1. Classify the change surface: rustdoc (`crates/**` comments), wiki (`wiki/*.md`), or repo docs (`docs/`).
2. Initialize/update the wiki submodule if touching wiki content (`git submodule update --init --recursive`).
3. Keep docs frontmatter complete and valid for `docs/*.md` changes.
4. Run `cargo xtask docs all`.
5. Run `cargo doc --workspace --no-deps` and `cargo test --workspace --doc` when rustdoc changed (recommended for all docs changes that mention APIs).
6. If Mermaid or OpenAPI changed, run targeted checks (`cargo xtask docs mermaid`, `cargo xtask docs openapi`) in addition to `all`.
7. Generate an audit artifact (`cargo xtask docs audit-report --output .artifacts/docs-audit.json`) when the change affects governance/reporting flows.

### 6.2 Code + docs changes

1. Update Rust code in the relevant crate(s).
2. Update rustdoc comments for affected public APIs in the same change.
3. Update affected wiki tutorials/how-to/explanations in `wiki/` when behavior/workflows/rationale changed.
4. Update affected governance docs/ADR/SOPs in `docs/` when process/contracts/architecture changed.
5. Run targeted Cargo checks (`cargo test --workspace` if behavior changed).
6. Run rustdoc checks (`cargo doc --workspace --no-deps`, `cargo test --workspace --doc`).
7. Run docs validation (`cargo xtask docs all`).

## 7) Key Files

- `xtask/src/docs.rs` (docs contract/integrity/audit CLI implementation)
- `tools/docs/doc_contracts.json` (docs schema/contract rules)
- `.gitmodules` (wiki submodule declaration)
- `wiki/` (GitHub Wiki submodule checkout)
- `docs/reference/rustdoc-and-github-wiki-documentation-strategy.md` (documentation surface split policy)
- `.cargo/config.toml` (Cargo aliases for local workflows)
- `Makefile` (optional convenience wrappers delegating to Cargo aliases)

## 8) Final Response Expectations (for agents)

In completion summaries:

- State what changed.
- List commands run (and whether they passed).
- Call out any checks not run (and why).
