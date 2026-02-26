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
  - MkDocs + Material (`docs/`, `mkdocs.yml`) for contracts, SOPs, ADRs, and docs tooling reference
  - Validation CLI in `scripts/docs/validate_docs.py` (MkDocs contracts + wiki structure checks)

## 2) Operating Rules

- Make minimal, reviewable changes that match existing patterns.
- If behavior, API shape, architecture, or procedures change, update docs in the same change.
- Preserve documentation contracts enforced by `tools/docs/doc_contracts.json`.
- Do not weaken validation rules or CI workflows unless explicitly requested.
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

## 4) CI Workflows (New / Current)

### 4.1 Documentation CI

Workflow: `.github/workflows/docs.yml`

Triggers:

- `pull_request`
- `push` to `main`

Stages (CI order):

1. Markdown lint (`markdownlint-cli2`)
2. Vale prose lint
3. Wiki submodule validation (`wiki`)
4. Docs contract validation (`structure`, `frontmatter`, `sop`)
5. OpenAPI validation (`openapi --require-validator`)
6. Mermaid render validation (`mermaid --require-renderer`)
7. Broken internal reference detection (`links`)
8. Rustdoc build (`cargo doc --workspace --no-deps`, `RUSTDOCFLAGS=-D warnings`)
9. Rustdoc doctests (`cargo test --workspace --doc`)
10. External link check (Lychee)
11. MkDocs build (`mkdocs build --strict`)

### 4.2 Quarterly Documentation Audit

Workflow: `.github/workflows/docs-audit.yml`

Triggers:

- Manual (`workflow_dispatch`)
- Scheduled quarterly (`0 9 1 */3 *`, 09:00 UTC on day 1 every 3rd month)

Behavior:

- Runs markdown/Vale checks
- Validates wiki submodule structure (via `audit-report`)
- Generates `.artifacts/docs-audit.json` via `audit-report`
- Builds MkDocs (best-effort / always-run)
- Uploads audit artifact
- Fails the job if audit validation fails

## 5) Local Commands

### 5.1 Docs Tooling Setup (match CI)

```bash
python -m pip install --upgrade pip
pip install mkdocs mkdocs-material pymdown-extensions
npm install -g markdownlint-cli2 @apidevtools/swagger-cli @mermaid-js/mermaid-cli
```

Optional local CLIs that CI also uses/actions wrap:

- `vale`
- `lychee`

### 5.2 Docs Validation (fast path)

Run the standard local docs validation entry point:

```bash
git submodule update --init --recursive
python3 scripts/docs/validate_docs.py all
cargo doc --workspace --no-deps
cargo test --workspace --doc
```

For stricter local parity when the CLIs are installed:

```bash
python3 scripts/docs/validate_docs.py all --require-renderer --require-openapi-validator
cargo doc --workspace --no-deps
cargo test --workspace --doc
```

### 5.3 CI-Parity Docs Commands (explicit)

```bash
git submodule update --init --recursive
markdownlint-cli2 "**/*.md"
vale docs
python3 scripts/docs/validate_docs.py structure
python3 scripts/docs/validate_docs.py wiki
python3 scripts/docs/validate_docs.py frontmatter
python3 scripts/docs/validate_docs.py sop
python3 scripts/docs/validate_docs.py openapi --require-validator
python3 scripts/docs/validate_docs.py mermaid --require-renderer
python3 scripts/docs/validate_docs.py links
cargo doc --workspace --no-deps
cargo test --workspace --doc
mkdocs build --strict
```

Optional external link check (if `lychee` is installed locally):

```bash
lychee --no-progress --verbose "docs/**/*.md"
```

### 5.4 Audit Report Command

```bash
python3 scripts/docs/validate_docs.py audit-report --output .artifacts/docs-audit.json
```

### 5.5 Rust Workspace Commands

Prefer direct Cargo commands for clarity (there is no `package.json` script wrapper in this repo). A root `Makefile` exists for convenience and mirrors common verification/docs/prototype commands:

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
```

Common convenience wrappers (delegating to Cargo aliases / docs scripts):

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

1. Classify the change surface: rustdoc (`crates/**` comments), wiki (`wiki/*.md`), or MkDocs (`docs/`).
2. Initialize/update the wiki submodule if touching wiki content (`git submodule update --init --recursive`).
3. Keep MkDocs frontmatter complete and valid for `docs/*.md` changes.
4. Run `python3 scripts/docs/validate_docs.py all`.
5. Run `cargo doc --workspace --no-deps` and `cargo test --workspace --doc` when rustdoc changed (recommended for all docs changes that mention APIs).
6. If Mermaid or OpenAPI changed, prefer strict checks with required CLIs.
7. Run `mkdocs build --strict` before finishing when `docs/` content changed.

### 6.2 Code + docs changes

1. Update Rust code in the relevant crate(s).
2. Update rustdoc comments for affected public APIs in the same change.
3. Update affected wiki tutorials/how-to/explanations in `wiki/` when behavior/workflows/rationale changed.
4. Update affected MkDocs governance docs/ADR/SOPs in `docs/` when process/contracts/architecture changed.
5. Run targeted Cargo checks (`cargo test --workspace` if behavior changed).
6. Run rustdoc checks (`cargo doc --workspace --no-deps`, `cargo test --workspace --doc`).
7. Run docs validation (`python3 scripts/docs/validate_docs.py all`).

## 7) Key Files

- `scripts/docs/validate_docs.py` (docs contract/integrity CLI)
- `tools/docs/doc_contracts.json` (docs schema/contract rules)
- `.gitmodules` (wiki submodule declaration)
- `wiki/` (GitHub Wiki submodule checkout)
- `docs/reference/rustdoc-and-github-wiki-documentation-strategy.md` (documentation surface split policy)
- `.github/workflows/docs.yml` (docs CI)
- `.github/workflows/docs-audit.yml` (quarterly audit workflow)
- `mkdocs.yml` (site build config)

## 8) Final Response Expectations (for agents)

In completion summaries:

- State what changed.
- List commands run (and whether they passed).
- Call out any checks not run (and why).
