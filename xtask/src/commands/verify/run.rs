use super::changed::{collect_changed_paths, looks_like_desktop_host_change};
use super::config::{VerifyFastDesktopMode, VerifyMode};
use crate::commands::dev::{wasm_target_installed, BuildWebCommand, CheckWebCommand};
use crate::commands::docs;
use crate::runtime::context::CommandContext;
use crate::runtime::error::XtaskResult;
use crate::XtaskCommand;

const DESKTOP_TAURI_PACKAGE: &str = "desktop_tauri";

#[derive(Clone, Debug)]
struct VerifyFastDesktopDecision {
    include_desktop: bool,
    reason: String,
}

pub(super) fn run_verify(
    ctx: &CommandContext,
    mode: VerifyMode,
    desktop_mode: VerifyFastDesktopMode,
) -> XtaskResult<()> {
    match mode {
        VerifyMode::Fast => verify_fast(ctx, desktop_mode),
        VerifyMode::Full => verify_full(ctx),
    }
}

fn resolve_verify_fast_desktop_decision(
    ctx: &CommandContext,
    mode: VerifyFastDesktopMode,
) -> VerifyFastDesktopDecision {
    match mode {
        VerifyFastDesktopMode::WithDesktop => VerifyFastDesktopDecision {
            include_desktop: true,
            reason: "forced by `--with-desktop`".to_string(),
        },
        VerifyFastDesktopMode::WithoutDesktop => VerifyFastDesktopDecision {
            include_desktop: false,
            reason: "forced by `--without-desktop`".to_string(),
        },
        VerifyFastDesktopMode::Auto => match collect_changed_paths(ctx) {
            Ok(paths) => infer_verify_fast_desktop_decision(paths),
            Err(err) => {
                ctx.workflow().warn(&format!(
                    "failed to inspect changed files for desktop auto-detection; including desktop host checks ({err})"
                ));
                VerifyFastDesktopDecision {
                    include_desktop: true,
                    reason: "automatic detection failed; defaulting to include desktop host checks"
                        .to_string(),
                }
            }
        },
    }
}

fn infer_verify_fast_desktop_decision(changed_paths: Vec<String>) -> VerifyFastDesktopDecision {
    if changed_paths.is_empty() {
        return VerifyFastDesktopDecision {
            include_desktop: false,
            reason: "no changed files detected by `git status --porcelain`".to_string(),
        };
    }

    let desktop_triggers: Vec<String> = changed_paths
        .iter()
        .filter(|path| looks_like_desktop_host_change(path))
        .cloned()
        .collect();

    if !desktop_triggers.is_empty() {
        return VerifyFastDesktopDecision {
            include_desktop: true,
            reason: format!(
                "desktop-relevant changes detected ({})",
                format_path_list_for_reason(&desktop_triggers)
            ),
        };
    }

    VerifyFastDesktopDecision {
        include_desktop: false,
        reason: "no desktop host trigger paths changed".to_string(),
    }
}

fn format_path_list_for_reason(paths: &[String]) -> String {
    if paths.len() <= 3 {
        return paths.join(", ");
    }

    let shown = paths[..3].join(", ");
    format!("{shown} (+{} more)", paths.len() - 3)
}

fn print_verify_fast_desktop_decision(decision: &VerifyFastDesktopDecision) {
    println!("\n==> Desktop host coverage (verify-fast)");
    if decision.include_desktop {
        println!("    included: {}", decision.reason);
    } else {
        println!("    skipped: {}", decision.reason);
        println!("    hint: pass `cargo verify-fast --with-desktop` to force desktop host checks");
    }
}

fn verify_fast(ctx: &CommandContext, desktop_mode: VerifyFastDesktopMode) -> XtaskResult<()> {
    let desktop_decision = resolve_verify_fast_desktop_decision(ctx, desktop_mode);
    print_verify_fast_desktop_decision(&desktop_decision);

    ctx.workflow()
        .run_timed_stage("Rust format and default test matrix", || {
            run_cargo_default_matrix_fast(ctx, desktop_decision.include_desktop)
        })?;
    ctx.workflow()
        .run_timed_stage("Rust all-features matrix", || {
            run_rust_feature_matrix_fast(ctx, desktop_decision.include_desktop)
        })?;
    ctx.workflow()
        .run_timed_stage("Rustdoc build and doctests", || {
            run_rustdoc_checks_fast(ctx, desktop_decision.include_desktop)
        })?;
    ctx.workflow()
        .run_timed_stage("Documentation validation and audit", || {
            run_docs_checks(ctx)
        })?;
    println!("\n==> Verification complete");
    Ok(())
}

fn verify_full(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.workflow()
        .run_timed_stage("Rust format and default test matrix", || {
            run_cargo_default_matrix_full(ctx)
        })?;
    ctx.workflow()
        .run_timed_stage("Rust all-features matrix", || {
            run_rust_feature_matrix_full(ctx)
        })?;
    ctx.workflow()
        .run_timed_stage("Rustdoc build and doctests", || {
            run_rustdoc_checks_full(ctx)
        })?;
    ctx.workflow()
        .run_timed_stage("Documentation validation and audit", || {
            run_docs_checks(ctx)
        })?;
    ctx.workflow()
        .run_timed_stage("Prototype compile checks", || {
            run_prototype_compile_checks(ctx)
        })?;
    ctx.workflow()
        .run_timed_stage("Clippy lint checks", || run_optional_clippy(ctx))?;
    println!("\n==> Verification complete");
    Ok(())
}

fn run_cargo_default_matrix_fast(ctx: &CommandContext, include_desktop: bool) -> XtaskResult<()> {
    ctx.process()
        .run(ctx.root(), "cargo", vec!["fmt", "--all", "--", "--check"])?;
    let mut test_args = vec!["test", "--workspace", "--lib", "--tests"];
    if !include_desktop {
        test_args.extend(["--exclude", DESKTOP_TAURI_PACKAGE]);
    }
    ctx.process().run(ctx.root(), "cargo", test_args)
}

fn run_cargo_default_matrix_full(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.process()
        .run(ctx.root(), "cargo", vec!["fmt", "--all", "--", "--check"])?;
    ctx.process()
        .run(ctx.root(), "cargo", vec!["check", "--workspace"])?;
    ctx.process().run(
        ctx.root(),
        "cargo",
        vec!["test", "--workspace", "--lib", "--tests"],
    )?;
    Ok(())
}

fn run_rust_feature_matrix_fast(ctx: &CommandContext, include_desktop: bool) -> XtaskResult<()> {
    let mut workspace_feature_test_args =
        vec!["test", "--workspace", "--all-features", "--lib", "--tests"];
    if !include_desktop {
        workspace_feature_test_args.extend(["--exclude", DESKTOP_TAURI_PACKAGE]);
    }

    ctx.process()
        .run(ctx.root(), "cargo", workspace_feature_test_args)
}

fn run_rust_feature_matrix_full(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.process().run(
        ctx.root(),
        "cargo",
        vec!["check", "--workspace", "--all-features"],
    )?;
    ctx.process().run(
        ctx.root(),
        "cargo",
        vec!["test", "--workspace", "--all-features", "--lib", "--tests"],
    )?;

    Ok(())
}

fn run_rustdoc_checks_fast(ctx: &CommandContext, include_desktop: bool) -> XtaskResult<()> {
    let mut doc_args = vec!["doc".into(), "--workspace".into(), "--no-deps".into()];
    if !include_desktop {
        doc_args.extend(["--exclude".into(), DESKTOP_TAURI_PACKAGE.into()]);
    }
    ctx.process().run_owned_with_env(
        ctx.root(),
        "cargo",
        doc_args,
        &[("RUSTDOCFLAGS", "-Dwarnings")],
    )?;

    let mut doc_test_args = vec!["test", "--workspace", "--doc"];
    if !include_desktop {
        doc_test_args.extend(["--exclude", DESKTOP_TAURI_PACKAGE]);
    }
    ctx.process().run(ctx.root(), "cargo", doc_test_args)?;
    Ok(())
}

fn run_rustdoc_checks_full(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.process().run_owned_with_env(
        ctx.root(),
        "cargo",
        vec!["doc".into(), "--workspace".into(), "--no-deps".into()],
        &[("RUSTDOCFLAGS", "-Dwarnings")],
    )?;
    ctx.process()
        .run(ctx.root(), "cargo", vec!["test", "--workspace", "--doc"])?;
    Ok(())
}

fn run_docs_checks(ctx: &CommandContext) -> XtaskResult<()> {
    docs::run_all(ctx)?;
    docs::write_report(ctx, ctx.artifacts().docs_audit_report())
}

fn run_optional_clippy(ctx: &CommandContext) -> XtaskResult<()> {
    let clippy_available = ctx.process().command_succeeds("cargo", &["clippy", "-V"]);

    if clippy_available {
        ctx.process().run(
            ctx.root(),
            "cargo",
            vec![
                "clippy",
                "--workspace",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
        )?;
    } else {
        ctx.workflow()
            .warn("cargo clippy not available; skipping clippy stage");
    }

    Ok(())
}

fn run_prototype_compile_checks(ctx: &CommandContext) -> XtaskResult<()> {
    CheckWebCommand::run(ctx, ())?;

    if wasm_target_installed() {
        BuildWebCommand::run(ctx, Vec::new())?;
    } else {
        ctx.workflow()
            .warn("wasm32-unknown-unknown target not installed; skipping trunk build");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_fast_auto_detection_includes_when_desktop_trigger_paths_changed() {
        let decision =
            infer_verify_fast_desktop_decision(vec!["crates/platform_host/src/lib.rs".into()]);
        assert!(decision.include_desktop);
    }

    #[test]
    fn verify_fast_auto_detection_skips_when_no_desktop_trigger_paths_changed() {
        let decision =
            infer_verify_fast_desktop_decision(vec!["crates/apps/notepad/src/lib.rs".into()]);
        assert!(!decision.include_desktop);
    }
}
