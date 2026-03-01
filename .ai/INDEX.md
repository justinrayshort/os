# AI Integration Index

**Version:** 1.0  
**Last Updated:** 2026-03-01  
**Purpose:** Single discovery point for Claude and other LLMs working in this repository.

This directory provides authoritative, machine-consumable context that enables deterministic, aligned code generation and analysis. All files are version-controlled and treated as normative references.

## Document Taxonomy

### Core Architectural Context
- **ARCHITECTURE.md** – High-level crate topology, boundaries, and dependency rules
- **HOST_BOUNDARY_CONTRACTS.md** – Typed contracts for browser/desktop/platform host separation
- **WORKSPACE_TOPOLOGY.md** – Crate inventory, purpose, dependencies, and maintainers

### Design System & UI Governance
- **DESIGN_TOKENS.md** – Theme tokens, semantic naming, and token consumption rules
- **UI_CONSTRAINTS.md** – Shared primitive enforcement, component contract, data-ui-kind rules
- **VISUAL_CONFORMANCE.md** – Apple HIG alignment, neumorphic standards, accessibility invariants

### Operational Contracts & Workflows
- **OPERATIONAL_CONTRACTS.md** – Input/output expectations, decision hierarchies, approval workflows
- **NAMING_CONVENTIONS.md** – Crate naming, module naming, function/type naming, file organization
- **AUTOMATION_COMMANDS.md** – Cargo aliases, xtask commands, verification workflows, CI/CD entry points
- **CODE_PATTERNS.md** – Idiomatic Rust patterns, error handling, testing conventions, rustdoc standards

### Validation & Enforcement
- **VALIDATION_RULES.md** – Architectural checks, forbidden patterns, linting rules, docs governance
- **GIT_WORKFLOW.md** – Commit message format, trailer requirements, branch policies

## How Claude Should Use These Documents

### Initial Context Loading
1. Read **ARCHITECTURE.md** to understand crate boundaries and dependencies
2. Read **OPERATIONAL_CONTRACTS.md** to understand constraints and decision rules
3. Skim **WORKSPACE_TOPOLOGY.md** to locate specific crates
4. Reference **NAMING_CONVENTIONS.md** and **CODE_PATTERNS.md** when writing code
5. Consult **DESIGN_TOKENS.md** and **UI_CONSTRAINTS.md** for any UI/shell changes

### Before Making Changes
1. Verify the change against **VALIDATION_RULES.md**
2. Check **ARCHITECTURE.md** for boundary violations
3. Review **NAMING_CONVENTIONS.md** if creating new modules/types
4. Confirm docs updates are required (see AGENTS.md section 6)

### During Code Review (Self-Check)
1. Validate against **CODE_PATTERNS.md** (rustdoc, error handling, testing)
2. Ensure no violations of **UI_CONSTRAINTS.md** (if UI/shell change)
3. Confirm commit message matches **GIT_WORKFLOW.md**
4. Verify documentation updates per **OPERATIONAL_CONTRACTS.md** (docs sync rule)

## Document Format & Conventions

All documents in `.ai/` follow these principles:
- **Deterministic:** No marketing language, subjective phrasing, or opinion
- **Concise:** Focused on actionable rules, not narrative explanation
- **Machine-readable:** Structured with clear headers, lists, and decision tables
- **Versioned:** Include version number and last-updated date
- **Indexed:** Link to authoritative sources (AGENTS.md, docs/, rustdoc)

## Authority Hierarchy

When guidance conflicts, apply in this order:
1. **AGENTS.md** (repository-wide operating rules, human-curated)
2. **.ai/* documents** (machine-consumable engineering directives)
3. **docs/** (formal artifacts, ADR/SOP/reference)
4. **rustdoc** (code-level API documentation)
5. **Wiki** (narrative explanation and tutorials)

## Maintenance

These files are maintained alongside code and documentation. When updating:
- Keep **ARCHITECTURE.md**, **HOST_BOUNDARY_CONTRACTS.md**, and **WORKSPACE_TOPOLOGY.md** in sync with crate changes
- Update **DESIGN_TOKENS.md** when theme tokens or semantic naming changes
- Refresh **AUTOMATION_COMMANDS.md** when new Cargo aliases or xtask commands are added
- Audit **VALIDATION_RULES.md** quarterly against actual architectural constraints

## Quick Links

- **Repository root:** /Users/justinshort/os/
- **Cargo workspace:** Cargo.toml
- **Repository governance:** AGENTS.md
- **Build verification:** `cargo verify-fast` or `cargo verify`
- **Docs validation:** `cargo xtask docs all`
- **UI conformance:** `cargo xtask docs ui-conformance`

## Related Resources

- **External Wiki:** GitHub Wiki repository (canonical narrative/architectural record)
- **Docs directory:** docs/ (formal artifacts, ADR/SOP/reference/tutorial/explanation)
- **Rustdoc:** `cargo doc --workspace --no-deps`
- **Linting:** `cargo clippy --workspace`, `cargo fmt --all`
