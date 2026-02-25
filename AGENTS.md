# AGENTS.md

This repository is maintained with help from automated agents. Use this file as the repo-specific operating guide.

## 1) Project Scope

- Rust workspace with two crates:
  - `crates/desktop_runtime`
  - `crates/site`
- Documentation system built with MkDocs + Material:
  - docs sources under `docs/`
  - config in `mkdocs.yml`
  - validation CLI in `scripts/docs/validate_docs.py`

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
3. Docs contract validation (`structure`, `frontmatter`, `sop`)
4. OpenAPI validation (`openapi --require-validator`)
5. Mermaid render validation (`mermaid --require-renderer`)
6. Broken internal reference detection (`links`)
7. External link check (Lychee)
8. MkDocs build (`mkdocs build --strict`)

### 4.2 Quarterly Documentation Audit

Workflow: `.github/workflows/docs-audit.yml`

Triggers:

- Manual (`workflow_dispatch`)
- Scheduled quarterly (`0 9 1 */3 *`, 09:00 UTC on day 1 every 3rd month)

Behavior:

- Runs markdown/Vale checks
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
python3 scripts/docs/validate_docs.py all
```

For stricter local parity when the CLIs are installed:

```bash
python3 scripts/docs/validate_docs.py all --require-renderer --require-openapi-validator
```

### 5.3 CI-Parity Docs Commands (explicit)

```bash
markdownlint-cli2 "**/*.md"
vale docs
python3 scripts/docs/validate_docs.py structure
python3 scripts/docs/validate_docs.py frontmatter
python3 scripts/docs/validate_docs.py sop
python3 scripts/docs/validate_docs.py openapi --require-validator
python3 scripts/docs/validate_docs.py mermaid --require-renderer
python3 scripts/docs/validate_docs.py links
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

Use direct Cargo commands (there is no root `Makefile` or `package.json` script wrapper in this repo):

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
```

## 6) Change Workflows for Agents

### 6.1 Docs-only changes

1. Edit files under `docs/` (and `docs/assets/` / OpenAPI assets if needed).
2. Keep frontmatter complete and valid.
3. Run `python3 scripts/docs/validate_docs.py all`.
4. If Mermaid or OpenAPI changed, prefer strict checks with required CLIs.
5. Run `mkdocs build --strict` before finishing.

### 6.2 Code + docs changes

1. Update Rust code in the relevant crate(s).
2. Update affected docs in the same change.
3. Run targeted Cargo checks (`cargo test --workspace` if behavior changed).
4. Run docs validation (`python3 scripts/docs/validate_docs.py all`).

## 7) Key Files

- `scripts/docs/validate_docs.py` (docs contract/integrity CLI)
- `tools/docs/doc_contracts.json` (docs schema/contract rules)
- `.github/workflows/docs.yml` (docs CI)
- `.github/workflows/docs-audit.yml` (quarterly audit workflow)
- `mkdocs.yml` (site build config)

## 8) Final Response Expectations (for agents)

In completion summaries:

- State what changed.
- List commands run (and whether they passed).
- Call out any checks not run (and why).
