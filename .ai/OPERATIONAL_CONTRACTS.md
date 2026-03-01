# Operational Contracts: AI Integration Rules

**Version:** 1.0  
**Last Updated:** 2026-03-01  
**Audience:** Claude, code generators, automation agents

## Core Directives

### 1. Documentation Synchronization (MANDATORY)

**Rule:** All code changes must include documentation updates in the same change.

**When a code change affects:**
| What Changed | Update These |
|---|---|
| Module/crate API shape, types, traits, functions | rustdoc (`///`, `//!`) at the changed location |
| Behavior, error semantics, or user workflows | Relevant Wiki pages (Explanation, How-to, Reference) |
| Architecture, boundaries, or system design | ARCHITECTURE.md, HOST_BOUNDARY_CONTRACTS.md, related Wiki Explanation pages |
| UI/shell design tokens, primitives, or conformance | DESIGN_TOKENS.md, VISUAL_CONFORMANCE.md, UI_CONSTRAINTS.md, checklist updates |
| Automation commands or workflows | AUTOMATION_COMMANDS.md, AGENTS.md section 5 |
| Governance, documentation format, or validation rules | VALIDATION_RULES.md, AGENTS.md section 3 |

**Before shipping:**
- Run `cargo verify-fast` to check docs and tests pass
- Run `cargo xtask docs all` to validate docs contracts
- Run `cargo xtask docs ui-conformance` if UI/theme changed
- If wiki content changed, commit wiki updates separately and record branch/SHA

### 2. Constraint Verification (MANDATORY BEFORE CHANGES)

**Rule:** Before writing code, verify against architectural constraints.

**Checklist:**
1. Read ARCHITECTURE.md crate topology for the crates you will touch
2. Confirm no forbidden dependencies (see ARCHITECTURE.md: Dependency Rules)
3. Check HOST_BOUNDARY_CONTRACTS.md if crossing browser/native boundary
4. Verify NAMING_CONVENTIONS.md alignment if creating new modules/types/functions
5. Review CODE_PATTERNS.md for idiomatic Rust, error handling, rustdoc expectations
6. If UI/shell change: verify against DESIGN_TOKENS.md and UI_CONSTRAINTS.md

### 3. Minimal, Surgical Changes (MANDATORY)

**Rule:** Make the smallest possible changes to address the request.

**What this means:**
- Don't refactor unrelated code unless explicitly requested
- Don't rewrite existing patterns; match established style
- Don't delete working code; only remove code explicitly marked for deletion
- Don't update documentation beyond scope of your change
- Preserve git history; use targeted edits, not wholesale rewrites

### 4. Self-Review Before Commit (MANDATORY)

**Before committing:**
1. **Validate architecture compliance:**
   - Check VALIDATION_RULES.md for any violations (e.g., forbidden imports, docs governance)
   - Ensure no circular dependencies or boundary crossings
   - Confirm rustdoc is present for public APIs (run `cargo doc --workspace --no-deps`)

2. **Verify documentation is current:**
   - Rustdoc matches implementation (no stale descriptions)
   - Wiki pages (if changed) preserve Diataxis intent separation
   - Commit message includes accurate description and footers

3. **Test and lint:**
   - Run `cargo verify-fast` (quick path) or `cargo verify` (full)
   - Run tests for affected crates: `cargo test --workspace`
   - Confirm compiler warnings are resolved or intentionally allowed

4. **Commit message format:**
   - Follow format in GIT_WORKFLOW.md
   - Include `Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>` trailer

### 5. When Uncertain, Escalate (DECISION HIERARCHY)

**Authority order for ambiguous cases:**
1. **AGENTS.md** (repo operating rules)
2. **This file** (.ai/OPERATIONAL_CONTRACTS.md)
3. **ARCHITECTURE.md** (crate topology)
4. **docs/** folder (formal artifacts, ADRs)
5. **Rustdoc** (code-level guidance)
6. **Wiki** (narrative explanation)

**If none of the above clarifies the decision, ask the user via `ask_user` tool.**

## Input/Output Expectations

### Claude Receives (Inputs)

1. **Code changes to review or implement:**
   - Specific files or functions to modify
   - Behavioral requirements or constraints
   - Architectural principles that must be honored

2. **Context from the environment:**
   - File contents via `view` tool
   - Build/test output via `bash`
   - Cargo metadata via `cargo metadata`
   - Git history via `git` command

3. **User guidance (explicit):**
   - Feature scope, design decisions, or constraints
   - Clarifications on ambiguous requirements
   - Approval for specific approaches

### Claude Produces (Outputs)

1. **Code changes:**
   - Minimal, targeted edits using `edit` or `create` tools
   - Preserved existing style and patterns
   - Complete rustdoc for public API changes
   - Test updates if behavior changed

2. **Documentation updates:**
   - Rustdoc updates in same commit as code changes
   - Wiki updates where behavioral/architectural changes require narrative updates
   - Updates to .ai/ documents only if fundamental rules/structure changed

3. **Verification:**
   - Run `cargo verify-fast` or full `cargo verify` before completing
   - Confirm no new warnings or test failures
   - Validate docs contracts pass (`cargo xtask docs all`)
   - Report test results and any checks not run (with justification)

4. **Commit message:**
   - Clear description of what changed and why
   - Reference related docs/issues if applicable
   - Include required Co-authored-by trailer

## Naming Conventions Reference

**For detailed naming rules, see NAMING_CONVENTIONS.md.** Quick rules:

- **Crates:** snake_case, plural when containing multiple items (crates/apps/)
- **Modules:** snake_case, match file organization
- **Types/Traits:** PascalCase, descriptive (CommandRegistry, DesktopState)
- **Functions:** snake_case, verb-forward (execute_command, parse_input)
- **Constants:** SCREAMING_SNAKE_CASE (BUFFER_SIZE)
- **Feature flags:** kebab-case (enable-perf-profiling)

## Error Handling Pattern

**All errors must be:**
1. Typed (Result<T, E> where E is a defined error type)
2. Documented (error semantics in rustdoc)
3. Propagated (don't panic; return error)

See CODE_PATTERNS.md for full error handling guidelines.

## Rustdoc Requirements

**All public APIs must have rustdoc:**
- Crate-level (`//!` module overview)
- Type/trait/function level (`///` item docs)
- Must include: summary, invariants/constraints, error semantics
- Should include: examples (for user-facing APIs), cross-references (intra-doc links)

Run `cargo doc --workspace --no-deps` to build and review.

## Testing Expectations

**For code changes:**
- Unit tests required for public API changes
- Integration tests if cross-crate behavior changed
- Run `cargo test --workspace` before committing

**For docs:**
- Doctests pass: `cargo test --workspace --doc`
- Docs contracts pass: `cargo xtask docs all`
- Links validated: `cargo xtask docs links`

## UI & Design System Changes

**Special handling for crates/system_ui, shell/theme changes:**

1. **Review DESIGN_TOKENS.md** for all token definitions and semantic naming
2. **Review UI_CONSTRAINTS.md** for primitive enforcement rules
3. **Run `cargo xtask docs ui-conformance`** to audit token/primitive hygiene
4. **Update VISUAL_CONFORMANCE.md** with evidence-based checklist status if conformance changed
5. **Follow docs/sop/ui-design-conformance-review-sop.md** for material design changes

## Wiki Updates (If Applicable)

**When updating Wiki content:**
1. Preserve Diataxis intent (Tutorial, How-to, Reference, Explanation)
2. For tutorials/how-to: preserve instructional template (Outcome, Entry Criteria, Procedure, Validation, Next Steps)
3. Update navigation pages (Home.md, _Sidebar.md) when adding new pages
4. Commit wiki changes to external wiki repo and reference the branch/SHA

## Automation Command Reference

For approved changes, the following commands validate your work:

| Command | Purpose |
|---|---|
| `cargo verify-fast` | Quick format, tests, docs build, docs contracts |
| `cargo verify` | Full matrix: format, tests, clippy, docs, profiles |
| `cargo xtask docs all` | Validate all docs contracts (structure, frontmatter, SOP, links) |
| `cargo xtask docs ui-conformance` | Audit UI token/primitive hygiene |
| `cargo doc --workspace --no-deps` | Build rustdoc; review for completeness |
| `cargo test --workspace --doc` | Run doctests |
| `cargo test --workspace` | Run all tests |
| `cargo fmt --all` | Format code |
| `cargo clippy --workspace --all-targets` | Lint checks |

All changes must pass `cargo verify-fast` at minimum before marking complete.

## Handling Unrelated Failures

**If tests/checks fail unrelated to your changes:**
- Don't fix them unless explicitly part of your task
- Document in the summary that they exist and why you didn't fix them
- Example: "Terminal app test fails due to pre-existing mock process issue; unrelated to this change"
