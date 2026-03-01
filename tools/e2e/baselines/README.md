# UI Feedback Baselines

This directory stores approved Playwright UI feedback baselines promoted via:

```bash
cargo e2e promote --profile <name> --source-run <run-id>
```

Each baseline lives under:

```text
tools/e2e/baselines/<scenario-id>/<slice-id>/<browser>/<viewport-id>/
```

Approved baseline directories contain:

- `screenshot.png`
- `dom.json`
- `a11y.json`
- `layout.json`
- `manifest.json`

Do not update these files manually. Regenerate candidate artifacts with `cargo e2e run`, review the captured diffs, and then promote the accepted run.
