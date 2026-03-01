# Git Workflow & Commit Standards

**Version:** 1.0  
**Last Updated:** 2026-03-01  
**Audience:** All contributors, code reviewers, automation agents

Defines commit message format, branch policies, and review expectations.

## Commit Message Format

### Structure

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Subject Line (Required)

- **Length:** 50 characters or less
- **Verb:** Imperative mood ("add", "fix", "update", not "adds", "added", "fixes")
- **Case:** Lowercase (except acronyms)
- **Period:** No trailing period
- **Reference:** Optionally include issue number (e.g., "fix(reducer): app launch #123")

**Examples:**
- ✅ `feat(system_ui): add FluentToggle primitive`
- ✅ `fix(platform_host_web): handle missing cache key`
- ✅ `docs(ARCHITECTURE.md): clarify app boundary rules`
- ❌ `Fixed a bug` (too vague)
- ❌ `Update code.` (too generic, has period)

### Type (Optional but Recommended)

Use conventional commit prefixes:

| Type | Purpose | Examples |
|---|---|---|
| feat | New feature | `feat(terminal): add history search` |
| fix | Bug fix | `fix(reducer): prevent app state corruption` |
| docs | Documentation update | `docs(rustdoc): clarify error semantics` |
| refactor | Code refactoring | `refactor(shell): extract command handler` |
| test | Test changes | `test(system_ui): add FluentButton tests` |
| perf | Performance improvement | `perf(desktop_runtime): optimize effect queue` |
| chore | Maintenance | `chore: update dependencies` |

### Scope (Optional but Recommended)

Identify affected crate/module:
- `feat(system_ui): ...`
- `fix(platform_host_web): ...`
- `docs(AGENTS.md): ...`

### Body (If Needed)

Explain **why** the change was made, not what. Wrap at 72 characters.

```
fix(reducer): prevent wallpaper load from blocking shell startup

Previously, loading a large wallpaper image in the reducer
could block the shell UI thread, causing jank. This change
moves wallpaper loading to the effects executor, allowing
the UI to remain responsive while the image loads asynchronously.

Fixes #456
```

### Footer (MANDATORY)

**Co-author trailer (required for all commits):**

```
Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

**Issue reference (if applicable):**

```
Fixes #123
Closes #456
Related-to #789
```

**Example full commit:**

```
feat(desktop_runtime): add app lifecycle hooks

Allows apps to hook into launch, focus, and shutdown events,
enabling proper initialization and cleanup. Each app receives
a LifecycleContext with access to host services and reducer
effects.

Refactored AppBus to support async lifecycle handlers.

Closes #234
Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

## Branch Policies

### Branch Naming

**Pattern:** `{type}/{short-description}`

Examples:
- `feat/add-wallpaper-transitions`
- `fix/cache-key-unicode-handling`
- `docs/update-architecture-guide`
- `refactor/shell-event-dispatch`

### Main Branch

**Protection:** Requires pull request + review

**Direct commits:** Forbidden (unless explicitly approved)

**Strategy:** Feature branches merged via squash or conventional commits

### Long-Running Branches

Not used in this repository; feature branches keep PRs focused and reviewable.

## Pull Request (Review) Workflow

### Before Requesting Review

1. **Branch off main** (latest main branch)
2. **Make minimal, focused changes** (one feature or fix per PR)
3. **Commit with clear messages** (per format above)
4. **Run local validation:**
   ```bash
   cargo verify-fast
   cargo verify
   ```
5. **Push to remote:** `git push origin feat/your-feature`
6. **Open PR** with clear description

### PR Description Template

```markdown
## What

Brief description of the change (one sentence).

## Why

Context and motivation for the change.

## How

Summary of implementation approach (if non-obvious).

## Validation

- [x] `cargo verify-fast` passes
- [x] `cargo verify` passes
- [x] Docs updated (rustdoc, Wiki, etc.)
- [x] Tests added/updated
- [x] No new warnings from clippy
- [ ] If UI changed: `cargo xtask docs ui-conformance` reviewed

## Checklist

- [x] Documentation is accurate and complete
- [x] No forbidden dependencies introduced
- [x] Commit messages follow conventional format
- [x] Code follows established patterns (NAMING_CONVENTIONS.md, CODE_PATTERNS.md)
```

### Review Expectations

**Reviewer checklist:**
1. Code follows naming and pattern conventions
2. Rustdoc is present and accurate
3. No architectural boundary violations
4. No forbidden dependencies
5. Tests cover new/changed logic
6. Docs (Wiki, ARCHITECTURE.md, etc.) are synchronized
7. Commit messages are clear and properly formatted

**Author responsibilities:**
1. Address all review comments
2. Re-request review after changes
3. Ensure CI passes before merge
4. Update PR description if scope changed

## Squashing & Merging

**Strategy:** Squash small commits into logical units; preserve meaningful history

**Examples:**
- ✅ 3 commits → 1 commit (fix typo, add test, update docs = 1 logical change)
- ✅ 5 commits → 3 commits (feat implementation, tests, docs = separate commits for clarity)
- ❌ 1 commit with 10 unrelated changes

**Guidelines:**
- Preserve commit messages that explain important decisions
- Squash fixup commits (corrections to previous commits)
- Preserve test/docs commits if they're logically separate

## Reverting & Fixing Mistakes

### If a Commit Breaks Something

1. **Notify maintainers immediately**
2. **Investigate root cause** (don't revert reflexively)
3. **If reverting:** `git revert COMMIT_SHA` (preserves history)
4. **If fixing forward:** New commit with fix (preferred if quick)

### Example Revert

```bash
git revert abc123def456
```

This creates a new commit that undoes abc123def456, preserving the history.

## Tag & Release Policy

**Not applicable to this development repository** (0.1.0 pre-release status). Tags are created by maintainers for releases.

## Destructive Operations (FORBIDDEN)

The following are **strictly forbidden** unless explicitly approved:

- ❌ `git push --force` or `git push -f` (rewrites history)
- ❌ `git reset --hard` on shared branches
- ❌ `git commit --amend` on already-pushed commits
- ❌ Deleting main or long-running branches

**If you need to amend a commit:** Create a new commit and request review again.

## Conflict Resolution

### Merge Conflicts

1. **Pull main:** `git pull origin main`
2. **Resolve conflicts** manually (editor or git tool)
3. **Test:** `cargo verify-fast` after resolution
4. **Commit merge:** `git commit -m "Merge main into feature/xyz"`
5. **Push:** `git push origin feat/your-feature`

### Rebasing

**Not recommended** for this workflow. Use merge commits instead (preserves history and avoids forced pushes).

## History Review

View clean commit history before merging:

```bash
git log --oneline origin/main..HEAD
```

Example output:
```
abc1234 feat(desktop_runtime): add app lifecycle hooks
def5678 test(desktop_runtime): add lifecycle hook tests
ghi9012 docs(ARCHITECTURE.md): update app integration section
```

Each commit should have:
- Clear subject line
- Optional body explaining why
- Proper footers (Co-authored-by, Fixes, etc.)

## Commit Hooks (Optional Local Setup)

If configured, pre-commit hooks validate:
- Message format
- No large files
- No secrets
- Code formatting

To bypass (not recommended): `git commit --no-verify`

## Sign-Off Policy

All commits should include the `Co-authored-by` trailer per AGENTS.md git trailer requirements:

```
Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
```

This is automatically applied by agents and should be verified by human reviewers.

## Troubleshooting

### Accidentally Committed to Main

```bash
# Create new branch from current position
git checkout -b feat/your-feature

# Reset main to previous state
git checkout main
git reset --hard origin/main

# Push feature branch
git push origin feat/your-feature
```

### Need to Undo Last Commit (Not Pushed)

```bash
git reset --soft HEAD~1
```

Then re-stage and re-commit with correct message.

### Large File Accidentally Committed

Contact maintainers; don't use `git rm --cached` and force push.

## Related Documentation

- AGENTS.md – Repository operating rules
- OPERATIONAL_CONTRACTS.md – Commit validation expectations
- NAMING_CONVENTIONS.md – Code naming standards
- CODE_PATTERNS.md – Rustdoc and code idioms
