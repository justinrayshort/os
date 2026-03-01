//! Shared process execution helpers.

use crate::runtime::cache::validate_sccache_config;
use crate::runtime::env::EnvHelper;
use crate::runtime::error::{XtaskError, XtaskResult};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Shared process runner used by command modules.
///
/// This type centralizes command execution style for xtask workflows:
/// - print commands in a stable `+ ...` format
/// - run from a caller-provided workspace-relative root
/// - normalize error categorization into [`XtaskError`]
/// - provide lightweight availability/probe helpers for workflow gating
///
/// It intentionally does not try to be a general process supervisor. Long-lived child lifecycle
/// management belongs in the dedicated runtime lifecycle helpers.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessRunner {
    env: EnvHelper,
}

impl ProcessRunner {
    /// Create a process runner.
    pub fn new() -> Self {
        Self { env: EnvHelper }
    }

    /// Return whether the given program is available by checking `--version`.
    ///
    /// Probe failures are treated as `false` instead of surfacing an error so callers can use this
    /// helper for prerequisite checks and optional-tool reporting.
    pub fn command_available(&self, program: &str) -> bool {
        Command::new(program)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Return whether the given cargo subcommand is available.
    pub fn cargo_subcommand_available(&self, subcommand: &str) -> bool {
        Command::new("cargo")
            .arg(subcommand)
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Return whether a command succeeds with the provided arguments.
    ///
    /// Stdout and stderr are suppressed. This helper is intended for probe-style checks rather than
    /// for workflows that should stream or preserve process output.
    pub fn command_succeeds(&self, program: &str, args: &[&str]) -> bool {
        Command::new(program)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Require a command to exist.
    ///
    /// Returns an environment error with the supplied hint when the command is unavailable.
    pub fn ensure_command(&self, program: &str, hint: &str) -> XtaskResult<()> {
        if self.command_available(program) {
            Ok(())
        } else {
            Err(XtaskError::environment(format!(
                "required command `{program}` not found. {hint}"
            )))
        }
    }

    /// Require a cargo subcommand to exist.
    pub fn ensure_cargo_subcommand(&self, subcommand: &str, hint: &str) -> XtaskResult<()> {
        if self.cargo_subcommand_available(subcommand) {
            Ok(())
        } else {
            Err(XtaskError::environment(format!(
                "required cargo subcommand `{subcommand}` not found. {hint}"
            )))
        }
    }

    /// Run a process with borrowed string arguments.
    ///
    /// This is a convenience wrapper over [`run_owned`](Self::run_owned).
    pub fn run(&self, root: &Path, program: &str, args: Vec<&str>) -> XtaskResult<()> {
        let owned = args.into_iter().map(ToString::to_string).collect();
        self.run_owned(root, program, owned)
    }

    /// Run a process with owned string arguments.
    ///
    /// The process inherits the terminal stdio streams. Non-zero exits are converted into
    /// [`XtaskError::process_exit`](crate::runtime::error::XtaskError::process_exit).
    pub fn run_owned(&self, root: &Path, program: &str, args: Vec<String>) -> XtaskResult<()> {
        self.print_command(program, &args);
        let mut cmd = Command::new(program);
        cmd.current_dir(root).args(&args);
        self.apply_process_contract(root, program, &mut cmd)?;
        let status = cmd.status().map_err(|err| {
            XtaskError::process_launch(format!("failed to start `{program}`: {err}"))
        })?;

        if status.success() {
            Ok(())
        } else {
            Err(XtaskError::process_exit(format!(
                "`{program}` exited with status {status}"
            )))
        }
    }

    /// Run a process with extra environment variables.
    ///
    /// Environment overrides are appended to the child process only for this invocation.
    pub fn run_owned_with_env(
        &self,
        root: &Path,
        program: &str,
        args: Vec<String>,
        envs: &[(&str, &str)],
    ) -> XtaskResult<()> {
        self.print_command(program, &args);
        let mut cmd = Command::new(program);
        cmd.current_dir(root).args(&args);
        for (key, value) in envs {
            cmd.env(key, value);
        }
        self.apply_process_contract(root, program, &mut cmd)?;
        let status = cmd.status().map_err(|err| {
            XtaskError::process_launch(format!("failed to start `{program}`: {err}"))
        })?;

        if status.success() {
            Ok(())
        } else {
            Err(XtaskError::process_exit(format!(
                "`{program}` exited with status {status}"
            )))
        }
    }

    /// Run trunk in the site directory with normalized environment.
    ///
    /// This normalizes `NO_COLOR` handling through [`EnvHelper`] so trunk invocations behave
    /// consistently across shells that export numeric or boolean values.
    pub fn run_trunk(&self, cwd: PathBuf, args: Vec<String>) -> XtaskResult<()> {
        self.print_command("trunk", &args);
        let mut cmd = Command::new("trunk");
        cmd.current_dir(cwd).args(&args);
        self.env.apply_no_color_override(&mut cmd);

        let status = cmd
            .status()
            .map_err(|err| XtaskError::process_launch(format!("failed to start `trunk`: {err}")))?;

        if status.success() {
            Ok(())
        } else {
            Err(XtaskError::process_exit(format!(
                "`trunk` exited with status {status}"
            )))
        }
    }

    /// Run the Tauri CLI with normalized environment.
    ///
    /// Tauri hooks ultimately delegate frontend work back into Cargo-managed commands, so keeping
    /// environment normalization here reduces drift between the direct and delegated paths.
    pub fn run_tauri_cli(&self, tauri_dir: &Path, args: Vec<String>) -> XtaskResult<()> {
        let workspace_root = tauri_dir
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| {
                XtaskError::environment("desktop_tauri path does not resolve to workspace root")
            })?;
        self.print_command("cargo", &args);
        let mut cmd = Command::new("cargo");
        cmd.current_dir(tauri_dir).args(&args);
        if let Some(value) =
            EnvHelper::normalized_no_color_value(std::env::var("NO_COLOR").ok().as_deref())
        {
            cmd.env("NO_COLOR", value);
        }
        self.apply_process_contract(workspace_root, "cargo", &mut cmd)?;

        let status = cmd.status().map_err(|err| {
            XtaskError::process_launch(format!("failed to start `cargo`: {err}"))
        })?;

        if status.success() {
            Ok(())
        } else {
            Err(XtaskError::process_exit(format!(
                "`cargo` exited with status {status}"
            )))
        }
    }

    /// Print a process invocation in a stable format.
    pub fn print_command(&self, program: &str, args: &[String]) {
        if args.is_empty() {
            println!("+ {program}");
        } else {
            println!("+ {program} {}", args.join(" "));
        }
    }

    /// Capture the first stdout line from a simple probe command.
    ///
    /// Returns `"unavailable"` when the command cannot be executed, exits unsuccessfully, or does
    /// not emit a first line of stdout.
    pub fn capture_stdout_line(&self, program: &str, args: &[&str]) -> String {
        let Ok(output) = Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
        else {
            return "unavailable".into();
        };
        if !output.status.success() {
            return "unavailable".into();
        }
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .next()
            .unwrap_or("unavailable")
            .trim()
            .to_string()
    }

    fn apply_process_contract(
        &self,
        root: &Path,
        program: &str,
        cmd: &mut Command,
    ) -> XtaskResult<()> {
        if program != "cargo" {
            return Ok(());
        }

        let status = validate_sccache_config(root, false)
            .map_err(|err| err.with_operation("cargo command preflight"))?;
        for (key, value) in status.config.env_pairs() {
            cmd.env(key, value);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_available_reports_missing_binary() {
        let runner = ProcessRunner::new();
        assert!(!runner.command_available("definitely-not-a-real-command-binary"));
    }

    #[test]
    fn command_succeeds_distinguishes_success_and_failure() {
        let runner = ProcessRunner::new();
        assert!(runner.command_succeeds("cargo", &["--version"]));
        assert!(!runner.command_succeeds("cargo", &["__definitely_invalid_subcommand__"]));
    }

    #[test]
    fn capture_stdout_line_returns_unavailable_for_failure() {
        let runner = ProcessRunner::new();
        assert_eq!(
            runner.capture_stdout_line("cargo", &["__definitely_invalid_subcommand__"]),
            "unavailable"
        );
    }

    #[test]
    fn capture_stdout_line_returns_first_stdout_line() {
        let runner = ProcessRunner::new();
        let line = runner.capture_stdout_line("cargo", &["--version"]);
        assert!(line.starts_with("cargo "));
    }
}
