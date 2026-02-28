//! Shared process execution helpers.

use crate::runtime::env::EnvHelper;
use crate::runtime::error::{XtaskError, XtaskResult};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Shared process runner used by command modules.
#[derive(Clone, Copy, Debug, Default)]
pub struct ProcessRunner {
    env: EnvHelper,
}

impl ProcessRunner {
    /// Create a process runner.
    pub fn new() -> Self {
        Self { env: EnvHelper }
    }

    /// Returns whether the given program is available by checking `--version`.
    pub fn command_available(&self, program: &str) -> bool {
        Command::new(program)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// Returns whether the given cargo subcommand is available.
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
    pub fn run(&self, root: &Path, program: &str, args: Vec<&str>) -> XtaskResult<()> {
        let owned = args.into_iter().map(ToString::to_string).collect();
        self.run_owned(root, program, owned)
    }

    /// Run a process with owned string arguments.
    pub fn run_owned(&self, root: &Path, program: &str, args: Vec<String>) -> XtaskResult<()> {
        self.print_command(program, &args);
        let status = Command::new(program)
            .current_dir(root)
            .args(&args)
            .status()
            .map_err(|err| {
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
    pub fn run_tauri_cli(&self, tauri_dir: &Path, args: Vec<String>) -> XtaskResult<()> {
        if let Some(value) =
            EnvHelper::normalized_no_color_value(std::env::var("NO_COLOR").ok().as_deref())
        {
            self.run_owned_with_env(tauri_dir, "cargo", args, &[("NO_COLOR", value)])
        } else {
            self.run_owned(tauri_dir, "cargo", args)
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
}
