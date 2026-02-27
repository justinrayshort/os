//! Workspace maintenance and developer workflow commands (`cargo xtask`).
//!
//! The `xtask` binary wraps common prototype, verification, and environment setup commands so the
//! repository can expose stable entrypoints through Cargo aliases.

mod docs;
mod perf;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEV_SERVER_DIR: &str = ".artifacts/dev-server";
const DEV_SERVER_STATE_FILE: &str = ".artifacts/dev-server/state.env";
const DEV_SERVER_LOG_FILE: &str = ".artifacts/dev-server/trunk.log";
const DEV_SERVER_DEFAULT_HOST: &str = "127.0.0.1";
const DEV_SERVER_DEFAULT_PORT: u16 = 8080;
const DEV_SERVER_START_POLL: Duration = Duration::from_secs(8);
const DEV_SERVER_STOP_TIMEOUT: Duration = Duration::from_secs(5);
const SITE_CARGO_FEATURE: &str = "csr";
const PLATFORM_STORAGE_PACKAGE: &str = "platform_storage";
const DESKTOP_TAURI_PACKAGE: &str = "desktop_tauri";
const AUTOMATION_RUNS_DIR: &str = ".artifacts/automation/runs";
const VERIFY_PROFILES_FILE: &str = "tools/automation/verify_profiles.toml";

fn main() -> ExitCode {
    let root = workspace_root();
    let mut args = env::args().skip(1);

    let Some(cmd) = args.next() else {
        print_usage();
        return ExitCode::from(2);
    };

    let rest: Vec<String> = args.collect();

    let result = match cmd.as_str() {
        "setup-web" => setup_web(&root),
        "dev" => dev_command(&root, rest),
        "build-web" => build_web(&root, rest),
        "check-web" => check_web(&root),
        "tauri" => tauri_command(&root, rest),
        "flow" => flow_command(&root, rest),
        "doctor" => doctor_command(&root, rest),
        "docs" => docs::run_docs_command(&root, rest),
        "perf" => perf::run_perf_command(&root, rest),
        "verify" => verify(&root, rest),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => Err(format!("unknown xtask command: {other}")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives under workspace root")
        .to_path_buf()
}

fn print_usage() {
    eprintln!(
        "Usage: cargo xtask <command> [args]\n\
         \n\
         Commands:\n\
           setup-web           Install wasm target and trunk (if missing)\n\
           dev [...]           Prototype dev workflow (serve/start/stop/status/restart/build)\n\
           build-web [args]    Build static web bundle with trunk\n\
           check-web           Run site compile checks (CSR native + wasm)\n\
            tauri [...]         Tauri desktop workflow (dev/build/check)\n\
            flow [...]          Run scoped inner-loop checks for changed packages/docs\n\
            doctor [--fix]      Validate local automation/tooling prerequisites\n\
            docs <subcommand>   Docs validation/audit commands (Rust-native)\n\
            perf <subcommand>   Performance benchmarks/profiling workflows\n\
            verify [fast|full] [--with-desktop|--without-desktop] [--profile <name>]\n\
                              Run standardized local verification workflow (default: full)\n"
    );
}

fn setup_web(root: &Path) -> Result<(), String> {
    run(
        root,
        "rustup",
        vec!["target", "add", "wasm32-unknown-unknown"],
    )?;

    if command_available("trunk") {
        println!("trunk already installed");
        return Ok(());
    }

    run(root, "cargo", vec!["install", "trunk"])
}

fn dev_command(root: &Path, args: Vec<String>) -> Result<(), String> {
    match args.first().map(String::as_str) {
        None => dev_server_foreground(root, Vec::new()),
        Some("serve") => dev_server_foreground(root, args[1..].to_vec()),
        Some("start") => dev_server_start(root, args[1..].to_vec()),
        Some("stop") => dev_server_stop(root, false),
        Some("status") => dev_server_status(root),
        Some("logs") => dev_server_logs(root, args[1..].to_vec()),
        Some("restart") => {
            dev_server_stop(root, true)?;
            dev_server_start(root, args[1..].to_vec())
        }
        Some("build") => dev_build(root, args[1..].to_vec()),
        Some("help" | "--help" | "-h") => {
            print_dev_usage();
            Ok(())
        }
        _ => dev_server_foreground(root, args),
    }
}

fn print_dev_usage() {
    eprintln!(
        "Usage: cargo dev [serve|start|stop|status|logs|restart|build] [args]\n\
         \n\
         Subcommands:\n\
           (default)           Start trunk dev server in foreground (defaults to --open)\n\
           serve [trunk args]  Same as default foreground mode\n\
           start [trunk args]  Start trunk dev server in background (no browser open by default)\n\
           stop                Stop the managed background dev server\n\
           status              Show managed dev server status\n\
           logs [--lines N]    Print recent managed dev server log output\n\
           restart [args]      Restart the managed background dev server\n\
           build [args]        Build a dev static bundle via trunk (non-release)\n\
         \n\
         Notes:\n\
           - `cargo dev stop` only manages servers started with `cargo dev start`.\n\
           - Development serve/build defaults disable SRI and file hashing unless explicitly set.\n\
           - Logs and state are stored under `.artifacts/dev-server/`.\n"
    );
}

fn dev_server_foreground(root: &Path, args: Vec<String>) -> Result<(), String> {
    ensure_command(
        "trunk",
        "Install it with `cargo setup-web` (or `cargo install trunk`)",
    )?;

    let parsed = parse_trunk_serve_args(args, true)?;
    run_trunk(site_dir(root), trunk_serve_args(&parsed))
}

fn dev_server_start(root: &Path, args: Vec<String>) -> Result<(), String> {
    ensure_command(
        "trunk",
        "Install it with `cargo setup-web` (or `cargo install trunk`)",
    )?;

    let parsed = parse_trunk_serve_args(args, false)?;

    if let Some(state) = read_dev_server_state(root)? {
        match inspect_managed_pid(state.pid)? {
            ManagedPidStatus::Managed => {
                return Err(format!(
                    "managed dev server already running (pid {}). Use `cargo dev stop` or `cargo dev restart`.",
                    state.pid
                ));
            }
            ManagedPidStatus::NotRunning => {
                eprintln!(
                    "warn: removing stale dev server state (pid {} not running)",
                    state.pid
                );
                remove_dev_server_state(root)?;
            }
            ManagedPidStatus::Unmanaged(command) => {
                eprintln!(
                    "warn: state pid {} belongs to a different process; cleaning managed state without signaling\n  command: {}",
                    state.pid, command
                );
                remove_dev_server_state(root)?;
            }
        }
    }

    let log_path = dev_server_log_path(root);
    let mut child = spawn_trunk_background(site_dir(root), trunk_serve_args(&parsed), &log_path)?;

    let state = DevServerState {
        pid: child.id(),
        host: parsed.host.clone(),
        port: parsed.port,
        started_unix_secs: unix_timestamp_secs(),
        log_path: log_path.clone(),
    };
    write_dev_server_state(root, &state)?;

    match wait_for_startup(&mut child, &state) {
        Ok(StartupState::Ready) => {
            println!(
                "managed dev server ready: {} (pid {}, log: {})",
                state.url(),
                state.pid,
                state.log_path.display()
            );
        }
        Ok(StartupState::Starting) => {
            println!(
                "managed dev server started (pid {}) and is still warming up; check `cargo dev status` or logs at {}",
                state.pid,
                state.log_path.display()
            );
        }
        Err(err) => {
            let _ = remove_dev_server_state(root);
            return Err(err);
        }
    }

    Ok(())
}

fn dev_server_status(root: &Path) -> Result<(), String> {
    let Some(state) = read_dev_server_state(root)? else {
        println!("managed dev server: not running (no state file)");
        return Ok(());
    };

    match inspect_managed_pid(state.pid)? {
        ManagedPidStatus::Managed => {
            let listening = is_port_open(&state.host, state.port);
            let phase = if listening {
                "listening"
            } else {
                "running (starting or not reachable yet)"
            };
            println!(
                "managed dev server: {} | pid {} | {} | log {}",
                phase,
                state.pid,
                state.url(),
                state.log_path.display()
            );
        }
        ManagedPidStatus::NotRunning => {
            println!(
                "managed dev server: stale state (pid {} not running) | last url {} | log {}",
                state.pid,
                state.url(),
                state.log_path.display()
            );
            remove_dev_server_state(root)?;
            println!("managed dev server: stale state removed");
        }
        ManagedPidStatus::Unmanaged(command) => {
            println!(
                "managed dev server: stale state (pid {} belongs to another process) | last url {} | log {}",
                state.pid,
                state.url(),
                state.log_path.display()
            );
            println!("  command: {command}");
            remove_dev_server_state(root)?;
            println!("managed dev server: stale state removed");
        }
    }
    Ok(())
}

fn dev_server_stop(root: &Path, quiet_if_missing: bool) -> Result<(), String> {
    let Some(state) = read_dev_server_state(root)? else {
        if !quiet_if_missing {
            println!("managed dev server: not running (no state file)");
        }
        return Ok(());
    };

    match inspect_managed_pid(state.pid)? {
        ManagedPidStatus::NotRunning => {
            println!(
                "managed dev server: stale state (pid {} not running); cleaning up state",
                state.pid
            );
            remove_dev_server_state(root)?;
            return Ok(());
        }
        ManagedPidStatus::Unmanaged(command) => {
            println!(
                "managed dev server: stale state (pid {} now belongs to another process); refusing to signal and cleaning state",
                state.pid
            );
            println!("  command: {command}");
            remove_dev_server_state(root)?;
            return Ok(());
        }
        ManagedPidStatus::Managed => {}
    }

    println!("stopping managed dev server pid {}...", state.pid);
    signal_terminate(state.pid)?;

    let deadline = Instant::now() + DEV_SERVER_STOP_TIMEOUT;
    while Instant::now() < deadline {
        if !process_exists(state.pid)? {
            remove_dev_server_state(root)?;
            println!("managed dev server stopped");
            return Ok(());
        }
        thread::sleep(Duration::from_millis(200));
    }

    eprintln!(
        "warn: pid {} did not exit after {:?}; sending SIGKILL",
        state.pid, DEV_SERVER_STOP_TIMEOUT
    );
    signal_kill(state.pid)?;

    let kill_deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < kill_deadline {
        if !process_exists(state.pid)? {
            remove_dev_server_state(root)?;
            println!("managed dev server stopped");
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    Err(format!(
        "failed to stop managed dev server pid {} (still running after SIGKILL)",
        state.pid
    ))
}

fn dev_build(root: &Path, args: Vec<String>) -> Result<(), String> {
    trunk_build(root, args, BuildProfile::Dev)
}

fn build_web(root: &Path, args: Vec<String>) -> Result<(), String> {
    trunk_build(root, args, BuildProfile::Release)
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct DevLogsOptions {
    lines: usize,
}

fn dev_server_logs(root: &Path, args: Vec<String>) -> Result<(), String> {
    let options = parse_dev_logs_options(args)?;
    let path = read_dev_server_state(root)?
        .map(|state| state.log_path)
        .unwrap_or_else(|| dev_server_log_path(root));

    if !path.exists() {
        return Err(format!(
            "no managed dev server log file found at {}",
            path.display()
        ));
    }

    let tail = read_log_tail_with_limit(&path, options.lines)?;
    println!(
        "managed dev server log tail ({} lines) from {}",
        options.lines,
        path.display()
    );
    if tail.trim().is_empty() {
        println!("(log is empty)");
    } else {
        println!("{tail}");
    }
    Ok(())
}

fn parse_dev_logs_options(args: Vec<String>) -> Result<DevLogsOptions, String> {
    let mut lines = 40usize;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--lines" | "-n" => {
                let Some(value) = args.get(i + 1) else {
                    return Err("missing value for `--lines`".to_string());
                };
                lines = parse_positive_usize(value, "--lines")?;
                i += 2;
            }
            "help" | "--help" | "-h" => {
                return Err(
                    "usage: cargo dev logs [--lines <N>]\nexample: cargo dev logs --lines 80"
                        .to_string(),
                );
            }
            other => {
                return Err(format!(
                    "unsupported `cargo dev logs` argument `{other}` (expected `--lines <N>`)"
                ));
            }
        }
    }

    Ok(DevLogsOptions { lines })
}

fn parse_positive_usize(value: &str, flag: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("invalid numeric value for `{flag}`: `{value}`"))?;
    if parsed == 0 {
        return Err(format!("`{flag}` must be greater than zero"));
    }
    Ok(parsed)
}

fn trunk_build(root: &Path, args: Vec<String>, profile: BuildProfile) -> Result<(), String> {
    ensure_command(
        "trunk",
        "Install it with `cargo setup-web` (or `cargo install trunk`)",
    )?;

    let mut trunk_args = vec!["build".to_string(), "index.html".to_string()];
    if profile == BuildProfile::Release {
        trunk_args.push("--release".to_string());
    }
    if !args_specify_dist(&args) {
        trunk_args.push("--dist".to_string());
        trunk_args.push(
            match profile {
                BuildProfile::Dev => "target/trunk-dev-dist",
                BuildProfile::Release => "target/trunk-dist",
            }
            .to_string(),
        );
    }
    if profile == BuildProfile::Dev {
        if !args_specify_filehash(&args) {
            trunk_args.push("--filehash=false".to_string());
        }
        if !args_specify_no_sri(&args) {
            trunk_args.push("--no-sri=true".to_string());
        }
    }
    trunk_args.extend(args);

    run_trunk(site_dir(root), trunk_args)
}

fn check_web(root: &Path) -> Result<(), String> {
    run(
        root,
        "cargo",
        vec!["check", "-p", "site", "--features", SITE_CARGO_FEATURE],
    )?;

    if wasm_target_installed() {
        run(
            root,
            "cargo",
            vec![
                "check",
                "-p",
                "site",
                "--target",
                "wasm32-unknown-unknown",
                "--features",
                SITE_CARGO_FEATURE,
            ],
        )?;
    } else {
        eprintln!(
            "warn: wasm32-unknown-unknown target not installed; skipping wasm check (run `cargo setup-web`)"
        );
    }

    Ok(())
}

fn tauri_command(root: &Path, args: Vec<String>) -> Result<(), String> {
    match args.first().map(String::as_str) {
        None | Some("dev") => tauri_dev(root, args.get(1..).unwrap_or(&[]).to_vec()),
        Some("build") => tauri_build(root, args[1..].to_vec()),
        Some("check") => tauri_check(root),
        Some("help" | "--help" | "-h") => {
            print_tauri_usage();
            Ok(())
        }
        Some(other) => Err(format!("unknown tauri subcommand: {other}")),
    }
}

fn print_tauri_usage() {
    eprintln!(
        "Usage: cargo xtask tauri [dev|build|check] [args]\n\
         \n\
         Subcommands:\n\
           dev [args]          Run `cargo tauri dev` from `crates/desktop_tauri/`\n\
           build [args]        Run `cargo tauri build` from `crates/desktop_tauri/`\n\
           check               Compile-check the Tauri crate (`cargo check -p desktop_tauri`)\n\
         \n\
         Notes:\n\
           - `tauri.conf.json` hooks delegate to `cargo dev serve/build` for the frontend pipeline.\n\
           - Install CLI with `cargo install tauri-cli --version '^2.0'`.\n"
    );
}

fn tauri_dev(root: &Path, args: Vec<String>) -> Result<(), String> {
    ensure_command(
        "trunk",
        "Install it with `cargo setup-web` (or `cargo install trunk`)",
    )?;
    ensure_cargo_subcommand(
        "tauri",
        "Install it with `cargo install tauri-cli --version '^2.0'`.",
    )?;

    let mut tauri_args = vec!["tauri".to_string(), "dev".to_string()];
    tauri_args.extend(args);
    run_tauri_cli(root, tauri_args)
}

fn tauri_build(root: &Path, args: Vec<String>) -> Result<(), String> {
    ensure_command(
        "trunk",
        "Install it with `cargo setup-web` (or `cargo install trunk`)",
    )?;
    ensure_cargo_subcommand(
        "tauri",
        "Install it with `cargo install tauri-cli --version '^2.0'`.",
    )?;

    let mut tauri_args = vec!["tauri".to_string(), "build".to_string()];
    tauri_args.extend(args);
    run_tauri_cli(root, tauri_args)
}

fn tauri_check(root: &Path) -> Result<(), String> {
    run(root, "cargo", vec!["check", "-p", "desktop_tauri"])
}

fn tauri_dir(root: &Path) -> PathBuf {
    root.join("crates/desktop_tauri")
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct DoctorOptions {
    fix: bool,
    show_help: bool,
}

fn doctor_command(root: &Path, args: Vec<String>) -> Result<(), String> {
    let options = parse_doctor_options(args)?;
    if options.show_help {
        print_doctor_usage();
        return Ok(());
    }

    with_workflow_run(root, "doctor", None, || run_doctor(root, options))
}

fn parse_doctor_options(args: Vec<String>) -> Result<DoctorOptions, String> {
    let mut options = DoctorOptions {
        fix: false,
        show_help: false,
    };

    for arg in args {
        match arg.as_str() {
            "--fix" => options.fix = true,
            "help" | "--help" | "-h" => options.show_help = true,
            other => {
                return Err(format!(
                    "unsupported `cargo doctor` argument `{other}` (expected `--fix`)"
                ));
            }
        }
    }
    Ok(options)
}

fn print_doctor_usage() {
    eprintln!(
        "Usage: cargo doctor [--fix]\n\
         \n\
         Purpose:\n\
           Validate local prerequisites for automated workflows and optionally repair safe issues.\n\
         \n\
         Options:\n\
           --fix                Attempt safe automatic remediation (stale state cleanup, wiki init, wasm target)\n"
    );
}

fn run_doctor(root: &Path, options: DoctorOptions) -> Result<(), String> {
    let mut failures = Vec::new();

    run_timed_stage("Tooling prerequisite: trunk", || {
        if command_available("trunk") {
            println!("    trunk is available");
            return Ok(());
        }
        if options.fix {
            println!("    trunk missing; running `cargo setup-web`");
            setup_web(root)?;
            if command_available("trunk") {
                println!("    trunk installed");
                return Ok(());
            }
        }
        Err("`trunk` is not available (run `cargo setup-web`)".to_string())
    })
    .map_err(|err| failures.push(err))
    .ok();

    run_timed_stage("Tooling prerequisite: wasm target", || {
        if wasm_target_installed() {
            println!("    wasm32-unknown-unknown target installed");
            return Ok(());
        }
        if options.fix {
            run(
                root,
                "rustup",
                vec!["target", "add", "wasm32-unknown-unknown"],
            )?;
            if wasm_target_installed() {
                println!("    wasm target installed");
                return Ok(());
            }
        }
        Err("wasm32-unknown-unknown target missing".to_string())
    })
    .map_err(|err| failures.push(err))
    .ok();

    run_timed_stage("Docs prerequisite: wiki submodule", || {
        let wiki_dot_git = root.join("wiki/.git");
        if wiki_dot_git.exists() {
            println!("    wiki submodule initialized");
            return Ok(());
        }
        if options.fix {
            run(
                root,
                "git",
                vec!["submodule", "update", "--init", "--recursive", "wiki"],
            )?;
            if wiki_dot_git.exists() {
                println!("    wiki submodule initialized");
                return Ok(());
            }
        }
        Err(
            "wiki submodule is not initialized (`git submodule update --init --recursive`)"
                .to_string(),
        )
    })
    .map_err(|err| failures.push(err))
    .ok();

    run_timed_stage("Dev server state hygiene", || {
        let Some(state) = read_dev_server_state(root)? else {
            println!("    no managed dev server state file");
            return Ok(());
        };
        match inspect_managed_pid(state.pid)? {
            ManagedPidStatus::Managed => {
                println!("    managed dev server state is healthy");
                Ok(())
            }
            ManagedPidStatus::NotRunning | ManagedPidStatus::Unmanaged(_) => {
                if options.fix {
                    remove_dev_server_state(root)?;
                    println!("    removed stale managed dev server state");
                    Ok(())
                } else {
                    Err("managed dev server state is stale (run `cargo dev stop` or `cargo doctor --fix`)".to_string())
                }
            }
        }
    })
    .map_err(|err| failures.push(err))
    .ok();

    if failures.is_empty() {
        println!("\n==> Doctor checks passed");
        return Ok(());
    }

    Err(format!(
        "doctor reported {} issue(s):\n- {}",
        failures.len(),
        failures.join("\n- ")
    ))
}

#[derive(Debug, Default)]
struct FlowOptions {
    packages: BTreeSet<String>,
    run_workspace: bool,
    force_docs: bool,
    skip_fmt: bool,
    show_help: bool,
}

#[derive(Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoMetadataPackage>,
    workspace_members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CargoMetadataPackage {
    id: String,
    name: String,
    manifest_path: String,
}

#[derive(Debug)]
struct WorkspacePackage {
    name: String,
    rel_dir: String,
}

fn flow_command(root: &Path, args: Vec<String>) -> Result<(), String> {
    let options = parse_flow_options(args)?;
    if options.show_help {
        print_flow_usage();
        return Ok(());
    }
    with_workflow_run(root, "flow", None, || flow_command_inner(root, options))
}

fn flow_command_inner(root: &Path, options: FlowOptions) -> Result<(), String> {
    let started = Instant::now();
    let changed_paths = if options.run_workspace || !options.packages.is_empty() {
        Vec::new()
    } else {
        collect_changed_paths(root)?
    };

    let docs_changed = changed_paths
        .iter()
        .any(|path| looks_like_docs_change(path));
    let rust_changed = changed_paths.iter().any(|path| path.ends_with(".rs"));
    let workspace_wide_change = changed_paths
        .iter()
        .any(|path| looks_like_workspace_wide_change(path));

    let workspace_packages = load_workspace_packages(root)?;
    let known_packages: HashSet<&str> = workspace_packages
        .iter()
        .map(|pkg| pkg.name.as_str())
        .collect();

    for package in &options.packages {
        if !known_packages.contains(package.as_str()) {
            return Err(format!(
                "unknown package `{package}` for `cargo flow`. Use `cargo metadata --no-deps` to list workspace packages."
            ));
        }
    }

    let scoped_packages = if !options.packages.is_empty() {
        options.packages.clone()
    } else {
        detect_changed_packages(&changed_paths, &workspace_packages)
    };
    let run_workspace_checks = options.run_workspace
        || workspace_wide_change
        || (scoped_packages.is_empty() && !docs_changed);

    if workspace_wide_change && !options.run_workspace {
        println!("workspace-wide change detected; running workspace flow checks");
    }

    if !options.run_workspace && options.packages.is_empty() && !run_workspace_checks {
        let package_list: Vec<String> = scoped_packages.iter().cloned().collect();
        println!("flow package scope: {}", format_package_list(&package_list));
    }

    if !options.run_workspace
        && options.packages.is_empty()
        && scoped_packages.is_empty()
        && docs_changed
    {
        println!("no crate changes detected; running docs-only flow checks");
    }

    if !options.skip_fmt
        && (rust_changed
            || run_workspace_checks
            || !scoped_packages.is_empty()
            || !options.packages.is_empty())
    {
        run_timed_stage("Rust format check", || {
            run(root, "cargo", vec!["fmt", "--all", "--", "--check"])
        })?;
    }

    if run_workspace_checks {
        run_timed_stage("Workspace compile check", || {
            run(root, "cargo", vec!["check", "--workspace"])
        })?;
        run_timed_stage("Workspace unit/integration tests", || {
            run(
                root,
                "cargo",
                vec!["test", "--workspace", "--lib", "--tests"],
            )
        })?;
    } else if !scoped_packages.is_empty() {
        let package_list: Vec<String> = scoped_packages.iter().cloned().collect();
        let check_label = format!(
            "Package compile check ({})",
            format_package_list(&package_list)
        );
        run_timed_stage(&check_label, || {
            run_owned(root, "cargo", cargo_check_package_args(&package_list))
        })?;

        let test_label = format!("Package tests ({})", format_package_list(&package_list));
        run_timed_stage(&test_label, || {
            run_owned(root, "cargo", cargo_test_package_args(&package_list))
        })?;
    }

    if options.force_docs || docs_changed || workspace_wide_change {
        run_timed_stage("Documentation validation", || {
            docs::run_docs_command(root, vec!["all".into()])
        })?;
    } else {
        println!("\n==> Documentation validation skipped (no docs/wiki/tooling changes detected)");
    }

    println!(
        "\n==> Flow checks complete in {}",
        format_duration(started.elapsed())
    );
    Ok(())
}

fn parse_flow_options(args: Vec<String>) -> Result<FlowOptions, String> {
    let mut options = FlowOptions::default();
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--all" => {
                options.run_workspace = true;
                i += 1;
            }
            "--docs" => {
                options.force_docs = true;
                i += 1;
            }
            "--no-fmt" => {
                options.skip_fmt = true;
                i += 1;
            }
            "-p" | "--package" => {
                let Some(package) = args.get(i + 1) else {
                    return Err("missing value for `--package`".to_string());
                };
                options.packages.insert(package.clone());
                i += 2;
            }
            "help" | "--help" | "-h" => {
                options.show_help = true;
                i += 1;
            }
            other => {
                return Err(format!(
                    "unsupported `cargo flow` argument `{other}` (run `cargo flow --help`)"
                ));
            }
        }
    }

    if options.run_workspace && !options.packages.is_empty() {
        return Err("`cargo flow --all` cannot be combined with `--package`".to_string());
    }

    Ok(options)
}

fn print_flow_usage() {
    eprintln!(
        "Usage: cargo flow [--all] [-p|--package <name> ...] [--docs] [--no-fmt]\n\
         \n\
         Purpose:\n\
           Run a fast inner-loop validation path scoped to changed workspace packages.\n\
         \n\
         Options:\n\
           --all                Run workspace-wide checks/tests\n\
           -p, --package <name> Restrict checks/tests to one or more explicit packages\n\
           --docs               Force docs validation (`cargo xtask docs all`)\n\
           --no-fmt             Skip `cargo fmt --all -- --check`\n\
         \n\
         Default behavior (no package/all flags):\n\
           - Detect changed files via `git status --porcelain`\n\
           - Run checks/tests only for changed workspace crates\n\
           - Run docs validation only when docs/wiki/tooling files changed\n"
    );
}

fn collect_changed_paths(root: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["status", "--porcelain"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("failed to run `git status --porcelain`: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "`git status --porcelain` exited with status {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    let mut paths = Vec::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(path) = parse_porcelain_status_path(line) {
            paths.push(path);
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn parse_porcelain_status_path(line: &str) -> Option<String> {
    let raw = line.get(3..)?.trim();
    if raw.is_empty() {
        return None;
    }

    let path = raw.rsplit(" -> ").next().unwrap_or(raw).trim();
    if path.is_empty() {
        return None;
    }

    if path.starts_with('"') && path.ends_with('"') && path.len() >= 2 {
        let unquoted = &path[1..path.len() - 1];
        return Some(unquoted.replace("\\\"", "\"").replace("\\\\", "\\"));
    }

    Some(path.to_string())
}

fn load_workspace_packages(root: &Path) -> Result<Vec<WorkspacePackage>, String> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("failed to run `cargo metadata`: {err}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "`cargo metadata` exited with status {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("failed to parse `cargo metadata` JSON: {err}"))?;
    let members: HashSet<&str> = metadata
        .workspace_members
        .iter()
        .map(String::as_str)
        .collect();

    let mut packages = Vec::new();
    for package in metadata.packages {
        if !members.contains(package.id.as_str()) {
            continue;
        }

        let manifest_path = PathBuf::from(package.manifest_path);
        let Some(manifest_dir) = manifest_path.parent() else {
            continue;
        };
        let rel_dir = manifest_dir
            .strip_prefix(root)
            .map(path_to_posix)
            .unwrap_or_else(|_| path_to_posix(manifest_dir));

        packages.push(WorkspacePackage {
            name: package.name,
            rel_dir,
        });
    }

    packages.sort_by(|a, b| {
        b.rel_dir
            .len()
            .cmp(&a.rel_dir.len())
            .then_with(|| a.name.cmp(&b.name))
    });
    Ok(packages)
}

fn path_to_posix(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(segment) => Some(segment.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn detect_changed_packages(
    changed_paths: &[String],
    workspace_packages: &[WorkspacePackage],
) -> BTreeSet<String> {
    let mut packages = BTreeSet::new();
    for path in changed_paths {
        for package in workspace_packages {
            if path == &package.rel_dir || path.starts_with(&format!("{}/", package.rel_dir)) {
                packages.insert(package.name.clone());
                break;
            }
        }
    }
    packages
}

fn looks_like_docs_change(path: &str) -> bool {
    path.starts_with("docs/")
        || path.starts_with("wiki/")
        || path == "README.md"
        || path == "AGENTS.md"
        || path == "tools/docs/doc_contracts.json"
        || path == "xtask/src/docs.rs"
}

fn looks_like_workspace_wide_change(path: &str) -> bool {
    matches!(
        path,
        "Cargo.toml" | "Cargo.lock" | ".cargo/config.toml" | "Makefile"
    ) || path == VERIFY_PROFILES_FILE
}

fn cargo_check_package_args(packages: &[String]) -> Vec<String> {
    let mut args = vec!["check".to_string()];
    for package in packages {
        args.push("-p".to_string());
        args.push(package.clone());
    }
    args
}

fn cargo_test_package_args(packages: &[String]) -> Vec<String> {
    let mut args = vec!["test".to_string()];
    for package in packages {
        args.push("-p".to_string());
        args.push(package.clone());
    }
    args
}

fn format_package_list(packages: &[String]) -> String {
    if packages.len() <= 4 {
        return packages.join(", ");
    }

    let shown = packages[..4].join(", ");
    format!("{shown} (+{} more)", packages.len() - 4)
}

fn verify(root: &Path, args: Vec<String>) -> Result<(), String> {
    let mut options = parse_verify_options(args)?;
    if options.show_help {
        let profiles = load_verify_profiles(root).ok();
        print_verify_usage(profiles.as_ref());
        return Ok(());
    }

    let profiles = load_verify_profiles(root)?;
    options = resolve_verify_options_from_profile(options, &profiles)?;
    if options.mode == VerifyMode::Full && options.desktop_mode != VerifyFastDesktopMode::Auto {
        return Err(
            "`--with-desktop` and `--without-desktop` are only valid with `cargo verify-fast`"
                .to_string(),
        );
    }

    let run_profile = options.profile.clone();
    with_workflow_run(root, "verify", run_profile, || match options.mode {
        VerifyMode::Fast => verify_fast(root, options.desktop_mode),
        VerifyMode::Full => verify_full(root),
    })
}

#[derive(Clone, Debug, Deserialize)]
struct VerifyProfilesFile {
    profile: BTreeSetProfileMap,
}

type BTreeSetProfileMap = std::collections::BTreeMap<String, VerifyProfileSpec>;

#[derive(Clone, Debug, Deserialize)]
struct VerifyProfileSpec {
    mode: String,
    desktop_mode: Option<String>,
}

fn load_verify_profiles(root: &Path) -> Result<BTreeSetProfileMap, String> {
    let path = root.join(VERIFY_PROFILES_FILE);
    let body = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let parsed: VerifyProfilesFile = toml::from_str(&body)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    if parsed.profile.is_empty() {
        return Err(format!("{} does not define any profiles", path.display()));
    }
    Ok(parsed.profile)
}

fn resolve_verify_profile(
    profile_name: &str,
    profiles: &BTreeSetProfileMap,
) -> Result<(VerifyMode, VerifyFastDesktopMode), String> {
    let Some(profile) = profiles.get(profile_name) else {
        let known = profiles.keys().cloned().collect::<Vec<_>>().join(", ");
        return Err(format!(
            "unknown verify profile `{profile_name}` (known: {known})"
        ));
    };

    let mode = match profile.mode.as_str() {
        "fast" => VerifyMode::Fast,
        "full" => VerifyMode::Full,
        other => {
            return Err(format!(
                "verify profile `{profile_name}` has invalid mode `{other}` (expected `fast` or `full`)"
            ))
        }
    };

    let desktop_mode = match profile.desktop_mode.as_deref().unwrap_or("auto") {
        "auto" => VerifyFastDesktopMode::Auto,
        "with-desktop" => VerifyFastDesktopMode::WithDesktop,
        "without-desktop" => VerifyFastDesktopMode::WithoutDesktop,
        other => {
            return Err(format!(
                "verify profile `{profile_name}` has invalid desktop_mode `{other}` (expected `auto`, `with-desktop`, `without-desktop`)"
            ))
        }
    };

    Ok((mode, desktop_mode))
}

fn print_verify_profile_selection(
    name: &str,
    mode: VerifyMode,
    desktop_mode: VerifyFastDesktopMode,
) {
    let mode_text = match mode {
        VerifyMode::Fast => "fast",
        VerifyMode::Full => "full",
    };
    let desktop_text = match desktop_mode {
        VerifyFastDesktopMode::Auto => "auto",
        VerifyFastDesktopMode::WithDesktop => "with-desktop",
        VerifyFastDesktopMode::WithoutDesktop => "without-desktop",
    };
    println!(
        "\n==> Verify profile selected: `{name}` (mode={mode_text}, desktop_mode={desktop_text})"
    );
}

fn resolve_verify_options_from_profile(
    mut options: VerifyOptions,
    profiles: &BTreeSetProfileMap,
) -> Result<VerifyOptions, String> {
    let Some(profile_name) = options.profile.clone() else {
        return Ok(options);
    };
    if options.explicit_mode {
        return Err(
            "`--profile` cannot be combined with `fast`/`full` positional mode".to_string(),
        );
    }
    if options.explicit_desktop_mode {
        return Err(
            "`--profile` cannot be combined with `--with-desktop`/`--without-desktop`".to_string(),
        );
    }
    let (mode, desktop_mode) = resolve_verify_profile(&profile_name, profiles)?;
    options.mode = mode;
    options.desktop_mode = desktop_mode;
    print_verify_profile_selection(&profile_name, mode, desktop_mode);
    Ok(options)
}

fn verify_profile_names(profiles: &BTreeSetProfileMap) -> String {
    profiles.keys().cloned().collect::<Vec<_>>().join(", ")
}

fn print_verify_usage(profiles: Option<&BTreeSetProfileMap>) {
    let profile_list = profiles
        .map(verify_profile_names)
        .unwrap_or_else(|| "<unavailable>".to_string());
    eprintln!(
        "Usage: cargo verify [fast|full] [--with-desktop|--without-desktop] [--profile <name>]\n\
         \n\
         Profiles:\n\
           {}\n\
         \n\
         Notes:\n\
           - `--profile` cannot be combined with explicit `fast`/`full` or desktop flags.\n\
           - desktop flags are only valid with `fast` mode.\n",
        profile_list
    );
}

fn parse_verify_options(args: Vec<String>) -> Result<VerifyOptions, String> {
    let mut options = VerifyOptions {
        mode: VerifyMode::Full,
        desktop_mode: VerifyFastDesktopMode::Auto,
        explicit_mode: false,
        explicit_desktop_mode: false,
        profile: None,
        show_help: false,
    };
    let mut i = 0usize;

    if let Some(first) = args.first().map(String::as_str) {
        match first {
            "fast" => {
                options.mode = VerifyMode::Fast;
                options.explicit_mode = true;
                i = 1;
            }
            "full" => {
                options.mode = VerifyMode::Full;
                options.explicit_mode = true;
                i = 1;
            }
            _ => {}
        }
    }

    while i < args.len() {
        match args[i].as_str() {
            "--with-desktop" => {
                if options.desktop_mode == VerifyFastDesktopMode::WithoutDesktop {
                    return Err(
                        "`--with-desktop` cannot be combined with `--without-desktop`".to_string(),
                    );
                }
                options.desktop_mode = VerifyFastDesktopMode::WithDesktop;
                options.explicit_desktop_mode = true;
                i += 1;
            }
            "--without-desktop" => {
                if options.desktop_mode == VerifyFastDesktopMode::WithDesktop {
                    return Err(
                        "`--with-desktop` cannot be combined with `--without-desktop`".to_string(),
                    );
                }
                options.desktop_mode = VerifyFastDesktopMode::WithoutDesktop;
                options.explicit_desktop_mode = true;
                i += 1;
            }
            "--profile" => {
                let Some(profile) = args.get(i + 1) else {
                    return Err("missing value for `--profile`".to_string());
                };
                options.profile = Some(profile.clone());
                i += 2;
            }
            "help" | "--help" | "-h" => {
                options.show_help = true;
                i += 1;
            }
            other => {
                return Err(format!(
                    "unsupported `cargo verify` argument `{other}` (expected `fast`, `full`, `--with-desktop`, `--without-desktop`, `--profile`)"
                ));
            }
        }
    }

    Ok(options)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VerifyMode {
    Fast,
    Full,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VerifyFastDesktopMode {
    Auto,
    WithDesktop,
    WithoutDesktop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct VerifyOptions {
    mode: VerifyMode,
    desktop_mode: VerifyFastDesktopMode,
    explicit_mode: bool,
    explicit_desktop_mode: bool,
    profile: Option<String>,
    show_help: bool,
}

#[derive(Clone, Debug)]
struct VerifyFastDesktopDecision {
    include_desktop: bool,
    reason: String,
}

fn resolve_verify_fast_desktop_decision(
    root: &Path,
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
        VerifyFastDesktopMode::Auto => match collect_changed_paths(root) {
            Ok(paths) => infer_verify_fast_desktop_decision(paths),
            Err(err) => {
                warn_stage(&format!(
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

fn looks_like_desktop_host_change(path: &str) -> bool {
    path.starts_with("crates/desktop_tauri/")
        || path.starts_with("crates/platform_host/")
        || path.starts_with("crates/platform_host_web/")
        || path.starts_with("crates/platform_storage/")
        || matches!(path, "Cargo.toml" | "Cargo.lock" | ".cargo/config.toml")
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

fn verify_fast(root: &Path, desktop_mode: VerifyFastDesktopMode) -> Result<(), String> {
    let started = Instant::now();
    let desktop_decision = resolve_verify_fast_desktop_decision(root, desktop_mode);
    print_verify_fast_desktop_decision(&desktop_decision);

    run_timed_stage("Rust format and default test matrix", || {
        run_cargo_default_matrix_fast(root, desktop_decision.include_desktop)
    })?;
    run_timed_stage("Rust all-features matrix", || {
        run_rust_feature_matrix_fast(root, desktop_decision.include_desktop)
    })?;
    run_timed_stage("Rustdoc build and doctests", || {
        run_rustdoc_checks_fast(root, desktop_decision.include_desktop)
    })?;
    run_timed_stage("Documentation validation and audit", || {
        run_docs_checks(root)
    })?;
    println!(
        "\n==> Verification complete in {}",
        format_duration(started.elapsed())
    );
    Ok(())
}

fn verify_full(root: &Path) -> Result<(), String> {
    let started = Instant::now();
    run_timed_stage("Rust format and default test matrix", || {
        run_cargo_default_matrix_full(root)
    })?;
    run_timed_stage("Rust all-features matrix", || {
        run_rust_feature_matrix_full(root)
    })?;
    run_timed_stage("Rustdoc build and doctests", || {
        run_rustdoc_checks_full(root)
    })?;
    run_timed_stage("Documentation validation and audit", || {
        run_docs_checks(root)
    })?;
    run_timed_stage("Prototype compile checks", || {
        run_prototype_compile_checks(root)
    })?;
    run_timed_stage("Clippy lint checks", || run_optional_clippy(root))?;
    println!(
        "\n==> Verification complete in {}",
        format_duration(started.elapsed())
    );
    Ok(())
}

fn run_cargo_default_matrix_fast(root: &Path, include_desktop: bool) -> Result<(), String> {
    run(root, "cargo", vec!["fmt", "--all", "--", "--check"])?;
    let mut test_args = vec!["test", "--workspace", "--lib", "--tests"];
    if !include_desktop {
        test_args.extend(["--exclude", DESKTOP_TAURI_PACKAGE]);
    }
    run(root, "cargo", test_args)?;
    Ok(())
}

fn run_cargo_default_matrix_full(root: &Path) -> Result<(), String> {
    run(root, "cargo", vec!["fmt", "--all", "--", "--check"])?;
    run(root, "cargo", vec!["check", "--workspace"])?;
    run(
        root,
        "cargo",
        vec!["test", "--workspace", "--lib", "--tests"],
    )?;
    Ok(())
}

fn run_rust_feature_matrix_fast(root: &Path, include_desktop: bool) -> Result<(), String> {
    let mut workspace_feature_test_args = vec![
        "test",
        "--workspace",
        "--all-features",
        "--exclude",
        PLATFORM_STORAGE_PACKAGE,
        "--lib",
        "--tests",
    ];
    if !include_desktop {
        workspace_feature_test_args.extend(["--exclude", DESKTOP_TAURI_PACKAGE]);
    }

    run(root, "cargo", workspace_feature_test_args)?;
    run(
        root,
        "cargo",
        vec![
            "test",
            "-p",
            PLATFORM_STORAGE_PACKAGE,
            "--no-default-features",
            "--features",
            "csr,desktop-host-stub",
            "--lib",
            "--tests",
        ],
    )?;

    run(
        root,
        "cargo",
        vec![
            "test",
            "-p",
            PLATFORM_STORAGE_PACKAGE,
            "--no-default-features",
            "--features",
            "csr,desktop-host-tauri",
            "--lib",
            "--tests",
        ],
    )?;

    Ok(())
}

fn run_rust_feature_matrix_full(root: &Path) -> Result<(), String> {
    run(
        root,
        "cargo",
        vec![
            "check",
            "--workspace",
            "--all-features",
            "--exclude",
            PLATFORM_STORAGE_PACKAGE,
        ],
    )?;
    run(
        root,
        "cargo",
        vec![
            "test",
            "--workspace",
            "--all-features",
            "--exclude",
            PLATFORM_STORAGE_PACKAGE,
            "--lib",
            "--tests",
        ],
    )?;

    run(
        root,
        "cargo",
        vec![
            "check",
            "-p",
            PLATFORM_STORAGE_PACKAGE,
            "--no-default-features",
            "--features",
            "csr,desktop-host-stub",
        ],
    )?;
    run(
        root,
        "cargo",
        vec![
            "test",
            "-p",
            PLATFORM_STORAGE_PACKAGE,
            "--no-default-features",
            "--features",
            "csr,desktop-host-stub",
            "--lib",
            "--tests",
        ],
    )?;

    run(
        root,
        "cargo",
        vec![
            "check",
            "-p",
            PLATFORM_STORAGE_PACKAGE,
            "--no-default-features",
            "--features",
            "csr,desktop-host-tauri",
        ],
    )?;
    run(
        root,
        "cargo",
        vec![
            "test",
            "-p",
            PLATFORM_STORAGE_PACKAGE,
            "--no-default-features",
            "--features",
            "csr,desktop-host-tauri",
            "--lib",
            "--tests",
        ],
    )?;

    Ok(())
}

fn run_rustdoc_checks_fast(root: &Path, include_desktop: bool) -> Result<(), String> {
    let mut doc_args = vec!["doc".into(), "--workspace".into(), "--no-deps".into()];
    if !include_desktop {
        doc_args.extend(["--exclude".into(), DESKTOP_TAURI_PACKAGE.into()]);
    }
    run_owned_with_env(root, "cargo", doc_args, &[("RUSTDOCFLAGS", "-Dwarnings")])?;

    let mut doc_test_args = vec!["test", "--workspace", "--doc"];
    if !include_desktop {
        doc_test_args.extend(["--exclude", DESKTOP_TAURI_PACKAGE]);
    }
    run(root, "cargo", doc_test_args)?;
    Ok(())
}

fn run_rustdoc_checks_full(root: &Path) -> Result<(), String> {
    run_owned_with_env(
        root,
        "cargo",
        vec!["doc".into(), "--workspace".into(), "--no-deps".into()],
        &[("RUSTDOCFLAGS", "-Dwarnings")],
    )?;
    run(root, "cargo", vec!["test", "--workspace", "--doc"])?;
    Ok(())
}

fn run_docs_checks(root: &Path) -> Result<(), String> {
    docs::run_docs_command(root, vec!["all".into()])?;
    docs::run_docs_command(
        root,
        vec![
            "audit-report".into(),
            "--output".into(),
            ".artifacts/docs-audit.json".into(),
        ],
    )?;
    Ok(())
}

fn run_optional_clippy(root: &Path) -> Result<(), String> {
    let clippy_available = Command::new("cargo")
        .args(["clippy", "-V"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    if clippy_available {
        run(
            root,
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
        warn_stage("cargo clippy not available; skipping clippy stage");
    }

    Ok(())
}

fn run_prototype_compile_checks(root: &Path) -> Result<(), String> {
    run(
        root,
        "cargo",
        vec!["check", "-p", "site", "--features", SITE_CARGO_FEATURE],
    )?;

    if wasm_target_installed() {
        run(
            root,
            "cargo",
            vec![
                "check",
                "-p",
                "site",
                "--target",
                "wasm32-unknown-unknown",
                "--features",
                SITE_CARGO_FEATURE,
            ],
        )?;
    } else {
        warn_stage("wasm32-unknown-unknown target not installed; skipping wasm cargo check");
    }

    if command_available("trunk") {
        trunk_build(root, Vec::new(), BuildProfile::Release)?;
    } else {
        warn_stage("trunk not installed; skipping trunk build");
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct AutomationStageRecord {
    name: String,
    started_unix_ms: u64,
    duration_ms: u128,
    status: String,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AutomationRunManifest {
    workflow: String,
    profile: Option<String>,
    started_unix_ms: u64,
    finished_unix_ms: u64,
    duration_ms: u128,
    status: String,
    error: Option<String>,
    run_dir: String,
    command: String,
    stages: Vec<AutomationStageRecord>,
}

#[derive(Debug)]
struct AutomationRunRecorder {
    workflow: String,
    profile: Option<String>,
    started_unix_ms: u64,
    started_instant: Instant,
    run_dir: PathBuf,
    manifest_path: PathBuf,
    events_path: PathBuf,
    command: String,
    stages: Vec<AutomationStageRecord>,
}

static ACTIVE_RUN_RECORDER: OnceLock<Mutex<Option<AutomationRunRecorder>>> = OnceLock::new();

fn active_run_recorder() -> &'static Mutex<Option<AutomationRunRecorder>> {
    ACTIVE_RUN_RECORDER.get_or_init(|| Mutex::new(None))
}

fn with_workflow_run<F>(
    root: &Path,
    workflow: &str,
    profile: Option<String>,
    action: F,
) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String>,
{
    let recorder = begin_workflow_run(root, workflow, profile)?;
    {
        let mut guard = active_run_recorder()
            .lock()
            .map_err(|_| "failed to lock workflow recorder".to_string())?;
        *guard = Some(recorder);
    }

    let result = action();
    finish_workflow_run(result.as_ref().err().cloned())?;
    result
}

fn begin_workflow_run(
    root: &Path,
    workflow: &str,
    profile: Option<String>,
) -> Result<AutomationRunRecorder, String> {
    let started_unix_ms = unix_timestamp_millis();
    let run_id = format!("{started_unix_ms}-{workflow}");
    let run_dir = root.join(AUTOMATION_RUNS_DIR).join(run_id);
    fs::create_dir_all(&run_dir)
        .map_err(|err| format!("failed to create {}: {err}", run_dir.display()))?;

    let events_path = run_dir.join("events.jsonl");
    let manifest_path = run_dir.join("manifest.json");
    fs::write(&events_path, "")
        .map_err(|err| format!("failed to initialize {}: {err}", events_path.display()))?;

    append_run_event(
        &events_path,
        serde_json::json!({
            "type": "run_started",
            "workflow": workflow,
            "profile": profile,
            "timestamp_unix_ms": started_unix_ms
        }),
    )?;

    Ok(AutomationRunRecorder {
        workflow: workflow.to_string(),
        profile,
        started_unix_ms,
        started_instant: Instant::now(),
        run_dir,
        manifest_path,
        events_path,
        command: env::args().collect::<Vec<_>>().join(" "),
        stages: Vec::new(),
    })
}

fn finish_workflow_run(error: Option<String>) -> Result<(), String> {
    let mut guard = active_run_recorder()
        .lock()
        .map_err(|_| "failed to lock workflow recorder".to_string())?;
    let Some(recorder) = guard.take() else {
        return Ok(());
    };

    let finished_unix_ms = unix_timestamp_millis();
    let status = if error.is_none() { "ok" } else { "failed" }.to_string();
    let manifest = AutomationRunManifest {
        workflow: recorder.workflow.clone(),
        profile: recorder.profile.clone(),
        started_unix_ms: recorder.started_unix_ms,
        finished_unix_ms,
        duration_ms: recorder.started_instant.elapsed().as_millis(),
        status: status.clone(),
        error: error.clone(),
        run_dir: recorder.run_dir.display().to_string(),
        command: recorder.command.clone(),
        stages: recorder.stages,
    };

    append_run_event(
        &recorder.events_path,
        serde_json::json!({
            "type": "run_finished",
            "workflow": recorder.workflow,
            "timestamp_unix_ms": finished_unix_ms,
            "status": status,
            "error": error
        }),
    )?;

    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|err| format!("failed to serialize automation manifest: {err}"))?;
    fs::write(&recorder.manifest_path, manifest_json).map_err(|err| {
        format!(
            "failed to write {}: {err}",
            recorder.manifest_path.display()
        )
    })?;
    println!(
        "    automation run artifact: {}",
        recorder.manifest_path.display()
    );

    Ok(())
}

fn append_run_event(path: &Path, event: serde_json::Value) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("failed to open {}: {err}", path.display()))?;
    let line = serde_json::to_string(&event)
        .map_err(|err| format!("failed to serialize run event: {err}"))?;
    use std::io::Write as _;
    writeln!(&mut file, "{line}")
        .map_err(|err| format!("failed to append {}: {err}", path.display()))
}

fn record_stage_event(
    stage: AutomationStageRecord,
    end_timestamp_unix_ms: u64,
) -> Result<(), String> {
    let mut guard = active_run_recorder()
        .lock()
        .map_err(|_| "failed to lock workflow recorder".to_string())?;
    let Some(recorder) = guard.as_mut() else {
        return Ok(());
    };
    append_run_event(
        &recorder.events_path,
        serde_json::json!({
            "type": "stage_finished",
            "name": stage.name,
            "started_unix_ms": stage.started_unix_ms,
            "finished_unix_ms": end_timestamp_unix_ms,
            "duration_ms": stage.duration_ms,
            "status": stage.status,
            "error": stage.error
        }),
    )?;
    recorder.stages.push(stage);
    Ok(())
}

fn warn_stage(message: &str) {
    println!("\n[warn] {message}");
}

fn run_timed_stage<F>(message: &str, action: F) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String>,
{
    println!("\n==> {message}");
    let started = Instant::now();
    let started_unix_ms = unix_timestamp_millis();
    match action() {
        Ok(()) => {
            let elapsed = started.elapsed();
            let stage = AutomationStageRecord {
                name: message.to_string(),
                started_unix_ms,
                duration_ms: elapsed.as_millis(),
                status: "ok".to_string(),
                error: None,
            };
            record_stage_event(stage, unix_timestamp_millis())?;
            println!("    done in {}", format_duration(elapsed));
            Ok(())
        }
        Err(err) => {
            let elapsed = started.elapsed();
            let stage = AutomationStageRecord {
                name: message.to_string(),
                started_unix_ms,
                duration_ms: elapsed.as_millis(),
                status: "failed".to_string(),
                error: Some(err.clone()),
            };
            record_stage_event(stage, unix_timestamp_millis())?;
            println!("    failed in {}", format_duration(elapsed));
            Err(err)
        }
    }
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    if secs >= 60 {
        let minutes = secs / 60;
        let rem_secs = secs % 60;
        format!("{minutes}m {rem_secs}.{millis:03}s")
    } else {
        format!("{secs}.{millis:03}s")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BuildProfile {
    Dev,
    Release,
}

#[derive(Clone, Debug)]
struct TrunkServeSpec {
    passthrough: Vec<String>,
    open: bool,
    host: String,
    port: u16,
}

#[derive(Clone, Debug)]
struct DevServerState {
    pid: u32,
    host: String,
    port: u16,
    started_unix_secs: u64,
    log_path: PathBuf,
}

impl DevServerState {
    fn url(&self) -> String {
        let host = if self.host == "0.0.0.0" {
            "127.0.0.1".to_string()
        } else if self.host == "::" {
            "[::1]".to_string()
        } else {
            self.host.clone()
        };
        format!("http://{host}:{}/", self.port)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartupState {
    Ready,
    Starting,
}

fn parse_trunk_serve_args(args: Vec<String>, default_open: bool) -> Result<TrunkServeSpec, String> {
    let mut passthrough = Vec::new();
    let mut open = default_open;
    let mut host = DEV_SERVER_DEFAULT_HOST.to_string();
    let mut port = DEV_SERVER_DEFAULT_PORT;

    let mut i = 0usize;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "--no-open" => {
                open = false;
                i += 1;
                continue;
            }
            "--open" => {
                open = true;
                i += 1;
                continue;
            }
            "--port" | "-p" => {
                let Some(value) = args.get(i + 1) else {
                    return Err(format!("missing value for `{arg}`"));
                };
                port = parse_port(value, arg)?;
                passthrough.push(arg.clone());
                passthrough.push(value.clone());
                i += 2;
                continue;
            }
            "--address" => {
                let Some(value) = args.get(i + 1) else {
                    return Err("missing value for `--address`".to_string());
                };
                host = value.clone();
                passthrough.push(arg.clone());
                passthrough.push(value.clone());
                i += 2;
                continue;
            }
            _ => {}
        }

        if let Some(value) = arg.strip_prefix("--port=") {
            port = parse_port(value, "--port")?;
            passthrough.push(arg.clone());
            i += 1;
            continue;
        }
        if let Some(value) = arg.strip_prefix("--address=") {
            host = value.to_string();
            passthrough.push(arg.clone());
            i += 1;
            continue;
        }
        if let Some(value) = arg.strip_prefix("-p") {
            if !value.is_empty() {
                port = parse_port(value, "-p")?;
                passthrough.push(arg.clone());
                i += 1;
                continue;
            }
        }

        passthrough.push(arg.clone());
        i += 1;
    }

    Ok(TrunkServeSpec {
        passthrough,
        open,
        host,
        port,
    })
}

fn parse_port(value: &str, flag: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .map_err(|_| format!("invalid port for `{flag}`: `{value}`"))
}

fn trunk_serve_args(spec: &TrunkServeSpec) -> Vec<String> {
    let mut trunk_args = vec!["serve".to_string(), "index.html".to_string()];
    if spec.open {
        trunk_args.push("--open".to_string());
    }
    if !args_specify_filehash(&spec.passthrough) {
        trunk_args.push("--filehash=false".to_string());
    }
    if !args_specify_no_sri(&spec.passthrough) {
        trunk_args.push("--no-sri=true".to_string());
    }
    trunk_args.extend(spec.passthrough.clone());
    trunk_args
}

fn args_specify_dist(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--dist" || arg.starts_with("--dist="))
}

fn args_specify_filehash(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--filehash" || arg.starts_with("--filehash="))
}

fn args_specify_no_sri(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--no-sri" || arg.starts_with("--no-sri="))
}

fn dev_server_state_path(root: &Path) -> PathBuf {
    root.join(DEV_SERVER_STATE_FILE)
}

fn dev_server_log_path(root: &Path) -> PathBuf {
    root.join(DEV_SERVER_LOG_FILE)
}

fn ensure_dev_server_dir(root: &Path) -> Result<PathBuf, String> {
    let path = root.join(DEV_SERVER_DIR);
    fs::create_dir_all(&path)
        .map_err(|err| format!("failed to create {}: {err}", path.display()))?;
    Ok(path)
}

fn write_dev_server_state(root: &Path, state: &DevServerState) -> Result<(), String> {
    ensure_dev_server_dir(root)?;
    let body = format!(
        "pid={}\nhost={}\nport={}\nstarted_unix_secs={}\nlog_path={}\n",
        state.pid,
        state.host,
        state.port,
        state.started_unix_secs,
        state.log_path.display()
    );

    fs::write(dev_server_state_path(root), body)
        .map_err(|err| format!("failed to write dev server state: {err}"))
}

fn read_dev_server_state(root: &Path) -> Result<Option<DevServerState>, String> {
    let path = dev_server_state_path(root);
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;

    let mut pid = None;
    let mut host = None;
    let mut port = None;
    let mut started_unix_secs = None;
    let mut log_path = None;

    for line in contents.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "pid" => pid = value.trim().parse::<u32>().ok(),
            "host" => host = Some(value.trim().to_string()),
            "port" => port = value.trim().parse::<u16>().ok(),
            "started_unix_secs" => started_unix_secs = value.trim().parse::<u64>().ok(),
            "log_path" => log_path = Some(PathBuf::from(value.trim())),
            _ => {}
        }
    }

    let state = DevServerState {
        pid: pid.ok_or_else(|| format!("invalid dev server state in {}", path.display()))?,
        host: host.unwrap_or_else(|| DEV_SERVER_DEFAULT_HOST.to_string()),
        port: port.unwrap_or(DEV_SERVER_DEFAULT_PORT),
        started_unix_secs: started_unix_secs.unwrap_or(0),
        log_path: log_path.unwrap_or_else(|| PathBuf::from(DEV_SERVER_LOG_FILE)),
    };

    Ok(Some(state))
}

fn remove_dev_server_state(root: &Path) -> Result<(), String> {
    let path = dev_server_state_path(root);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("failed to remove {}: {err}", path.display())),
    }
}

fn spawn_trunk_background(
    cwd: PathBuf,
    args: Vec<String>,
    log_path: &Path,
) -> Result<Child, String> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }

    let log = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path)
        .map_err(|err| format!("failed to open {}: {err}", log_path.display()))?;
    let log_out = log
        .try_clone()
        .map_err(|err| format!("failed to clone log file handle: {err}"))?;

    print_command("trunk", &args);
    let mut cmd = Command::new("trunk");
    cmd.current_dir(cwd)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_out))
        .stderr(Stdio::from(log));

    apply_no_color_override(&mut cmd);

    cmd.spawn()
        .map_err(|err| format!("failed to start `trunk` in background: {err}"))
}

fn wait_for_startup(child: &mut Child, state: &DevServerState) -> Result<StartupState, String> {
    let deadline = Instant::now() + DEV_SERVER_START_POLL;

    loop {
        if is_port_open(&state.host, state.port) {
            return Ok(StartupState::Ready);
        }

        if let Some(status) = child
            .try_wait()
            .map_err(|err| format!("failed while checking dev server startup: {err}"))?
        {
            let mut msg = format!("managed dev server exited during startup with status {status}");
            let tail = read_log_tail(&state.log_path, 20);
            if !tail.is_empty() {
                msg.push_str(&format!(
                    "\nlog tail ({}):\n{}",
                    state.log_path.display(),
                    tail
                ));
            }
            return Err(msg);
        }

        if Instant::now() >= deadline {
            return Ok(StartupState::Starting);
        }

        thread::sleep(Duration::from_millis(200));
    }
}

fn is_port_open(host: &str, port: u16) -> bool {
    let host = match host {
        "0.0.0.0" => "127.0.0.1",
        "::" => "::1",
        other => other,
    };

    let addr = format!("{host}:{port}");
    let Ok(addrs) = addr.to_socket_addrs() else {
        return false;
    };

    addrs.into_iter().any(|socket_addr| {
        TcpStream::connect_timeout(&socket_addr, Duration::from_millis(250)).is_ok()
    })
}

fn read_log_tail(path: &Path, max_lines: usize) -> String {
    read_log_tail_with_limit(path, max_lines).unwrap_or_default()
}

fn read_log_tail_with_limit(path: &Path, max_lines: usize) -> Result<String, String> {
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let lines: Vec<&str> = contents.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    Ok(lines[start..].join("\n"))
}

fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn unix_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn wasm_target_installed() -> bool {
    let Ok(output) = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    else {
        return false;
    };

    if !output.status.success() {
        return false;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.trim() == "wasm32-unknown-unknown")
}

fn command_available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn cargo_subcommand_available(subcommand: &str) -> bool {
    Command::new("cargo")
        .arg(subcommand)
        .arg("--help")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn ensure_cargo_subcommand(subcommand: &str, hint: &str) -> Result<(), String> {
    if cargo_subcommand_available(subcommand) {
        Ok(())
    } else {
        Err(format!(
            "required cargo subcommand `{subcommand}` not found. {hint}"
        ))
    }
}

#[cfg(unix)]
unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
}

#[cfg(unix)]
const SIGTERM: i32 = 15;
#[cfg(unix)]
const SIGKILL: i32 = 9;
#[cfg(unix)]
const EPERM: i32 = 1;
#[cfg(unix)]
const ESRCH: i32 = 3;

fn process_exists(pid: u32) -> Result<bool, String> {
    #[cfg(unix)]
    {
        match signal(pid, 0) {
            Ok(()) => Ok(true),
            Err(err) => match err.raw_os_error() {
                Some(ESRCH) => Ok(false),
                Some(EPERM) => Ok(true),
                _ => Err(format!("failed to query pid {pid}: {err}")),
            },
        }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        Err("managed dev server status/stop is only supported on unix hosts".to_string())
    }
}

#[derive(Debug, Clone)]
enum ManagedPidStatus {
    NotRunning,
    Managed,
    Unmanaged(String),
}

fn inspect_managed_pid(pid: u32) -> Result<ManagedPidStatus, String> {
    if !process_exists(pid)? {
        return Ok(ManagedPidStatus::NotRunning);
    }

    #[cfg(unix)]
    {
        let Some(command_line) = process_command_line(pid)? else {
            return Ok(ManagedPidStatus::NotRunning);
        };
        if looks_like_trunk_serve_command(&command_line) {
            Ok(ManagedPidStatus::Managed)
        } else {
            Ok(ManagedPidStatus::Unmanaged(command_line))
        }
    }

    #[cfg(not(unix))]
    {
        Ok(ManagedPidStatus::Managed)
    }
}

fn looks_like_trunk_serve_command(command: &str) -> bool {
    command.contains("trunk") && command.contains("serve")
}

#[cfg(unix)]
fn process_command_line(pid: u32) -> Result<Option<String>, String> {
    let output = Command::new("ps")
        .args(["-o", "command=", "-p", &pid.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| format!("failed to inspect pid {pid} with `ps`: {err}"))?;

    if !output.status.success() {
        return Ok(None);
    }

    let command_line = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if command_line.is_empty() {
        Ok(None)
    } else {
        Ok(Some(command_line))
    }
}

fn signal_terminate(pid: u32) -> Result<(), String> {
    signal_named(pid, SignalKind::Terminate)
}

fn signal_kill(pid: u32) -> Result<(), String> {
    signal_named(pid, SignalKind::Kill)
}

#[derive(Clone, Copy, Debug)]
enum SignalKind {
    Terminate,
    Kill,
}

fn signal_named(pid: u32, kind: SignalKind) -> Result<(), String> {
    #[cfg(unix)]
    {
        let sig = match kind {
            SignalKind::Terminate => SIGTERM,
            SignalKind::Kill => SIGKILL,
        };
        match signal(pid, sig) {
            Ok(()) => Ok(()),
            Err(err) if err.raw_os_error() == Some(ESRCH) => Ok(()),
            Err(err) => Err(format!("failed to signal pid {pid}: {err}")),
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, kind);
        Err("managed dev server stop is only supported on unix hosts".to_string())
    }
}

#[cfg(unix)]
fn signal(pid: u32, sig: i32) -> Result<(), io::Error> {
    let pid_i32 = i32::try_from(pid)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "pid out of range"))?;
    // SAFETY: libc kill is called with a validated pid and signal number.
    let rc = unsafe { kill(pid_i32, sig) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn ensure_command(program: &str, hint: &str) -> Result<(), String> {
    if command_available(program) {
        Ok(())
    } else {
        Err(format!("required command `{program}` not found. {hint}"))
    }
}

fn run(root: &Path, program: &str, args: Vec<&str>) -> Result<(), String> {
    let owned = args.into_iter().map(ToString::to_string).collect();
    run_owned(root, program, owned)
}

fn run_owned(root: &Path, program: &str, args: Vec<String>) -> Result<(), String> {
    print_command(program, &args);
    let status = Command::new(program)
        .current_dir(root)
        .args(&args)
        .status()
        .map_err(|err| format!("failed to start `{program}`: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("`{program}` exited with status {status}"))
    }
}

fn run_owned_with_env(
    root: &Path,
    program: &str,
    args: Vec<String>,
    envs: &[(&str, &str)],
) -> Result<(), String> {
    print_command(program, &args);
    let mut cmd = Command::new(program);
    cmd.current_dir(root).args(&args);
    for (key, value) in envs {
        cmd.env(key, value);
    }
    let status = cmd
        .status()
        .map_err(|err| format!("failed to start `{program}`: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("`{program}` exited with status {status}"))
    }
}

fn run_tauri_cli(root: &Path, args: Vec<String>) -> Result<(), String> {
    if let Some(value) = normalized_no_color_value(env::var("NO_COLOR").ok().as_deref()) {
        run_owned_with_env(&tauri_dir(root), "cargo", args, &[("NO_COLOR", value)])
    } else {
        run_owned(&tauri_dir(root), "cargo", args)
    }
}

fn run_trunk(cwd: PathBuf, args: Vec<String>) -> Result<(), String> {
    print_command("trunk", &args);
    let mut cmd = Command::new("trunk");
    cmd.current_dir(cwd).args(&args);

    apply_no_color_override(&mut cmd);

    let status = cmd
        .status()
        .map_err(|err| format!("failed to start `trunk`: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("`trunk` exited with status {status}"))
    }
}

fn site_dir(root: &Path) -> PathBuf {
    root.join("crates/site")
}

fn apply_no_color_override(cmd: &mut Command) {
    if let Some(value) = normalized_no_color_value(env::var("NO_COLOR").ok().as_deref()) {
        // Some environments export NO_COLOR=1, but trunk's CLI parser expects true/false.
        cmd.env("NO_COLOR", value);
    }
}

fn normalized_no_color_value(raw: Option<&str>) -> Option<&'static str> {
    match raw {
        Some("1") => Some("true"),
        _ => None,
    }
}

fn print_command(program: &str, args: &[String]) {
    if args.is_empty() {
        println!("+ {program}");
        return;
    }

    println!("+ {program} {}", args.join(" "));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_profiles() -> BTreeSetProfileMap {
        let mut profiles = std::collections::BTreeMap::new();
        profiles.insert(
            "dev".to_string(),
            VerifyProfileSpec {
                mode: "fast".to_string(),
                desktop_mode: Some("auto".to_string()),
            },
        );
        profiles.insert(
            "ci-full".to_string(),
            VerifyProfileSpec {
                mode: "full".to_string(),
                desktop_mode: Some("auto".to_string()),
            },
        );
        profiles
    }

    #[test]
    fn parse_foreground_dev_args_defaults_to_open() {
        let spec = parse_trunk_serve_args(Vec::new(), true).expect("parse");
        assert!(spec.open);
        assert_eq!(spec.host, DEV_SERVER_DEFAULT_HOST);
        assert_eq!(spec.port, DEV_SERVER_DEFAULT_PORT);
    }

    #[test]
    fn parse_managed_dev_args_tracks_port_and_address() {
        let spec = parse_trunk_serve_args(
            vec![
                "--address".into(),
                "0.0.0.0".into(),
                "--port=9001".into(),
                "--no-autoreload".into(),
            ],
            false,
        )
        .expect("parse");

        assert!(!spec.open);
        assert_eq!(spec.host, "0.0.0.0");
        assert_eq!(spec.port, 9001);
        assert!(spec.passthrough.contains(&"--no-autoreload".to_string()));
    }

    #[test]
    fn trunk_serve_args_default_to_no_sri_and_no_filehash() {
        let spec = parse_trunk_serve_args(Vec::new(), false).expect("parse");
        let args = trunk_serve_args(&spec);
        assert!(args.contains(&"--filehash=false".to_string()));
        assert!(args.contains(&"--no-sri=true".to_string()));
    }

    #[test]
    fn trunk_serve_args_respect_explicit_hash_and_sri_settings() {
        let spec = parse_trunk_serve_args(
            vec!["--filehash=true".into(), "--no-sri=false".into()],
            false,
        )
        .expect("parse");
        let args = trunk_serve_args(&spec);
        assert!(!args.contains(&"--filehash=false".to_string()));
        assert!(!args.contains(&"--no-sri=true".to_string()));
        assert!(args.contains(&"--filehash=true".to_string()));
        assert!(args.contains(&"--no-sri=false".to_string()));
    }

    #[test]
    fn dist_detection_handles_split_and_inline_forms() {
        assert!(args_specify_dist(&["--dist".into(), "x".into()]));
        assert!(args_specify_dist(&["--dist=target/custom".into()]));
        assert!(!args_specify_dist(&["--release".into()]));
    }

    #[test]
    fn logs_option_parser_defaults_and_rejects_zero() {
        let options = parse_dev_logs_options(Vec::new()).expect("parse");
        assert_eq!(options, DevLogsOptions { lines: 40 });

        let options =
            parse_dev_logs_options(vec!["--lines".into(), "80".into()]).expect("parse lines");
        assert_eq!(options, DevLogsOptions { lines: 80 });

        let err = parse_dev_logs_options(vec!["--lines".into(), "0".into()])
            .expect_err("zero should be rejected");
        assert!(err.contains("greater than zero"));
    }

    #[test]
    fn flow_option_parser_rejects_package_with_all_scope() {
        let err = parse_flow_options(vec!["--all".into(), "--package".into(), "site".into()])
            .expect_err("must reject incompatible flags");
        assert!(err.contains("--all"));
    }

    #[test]
    fn porcelain_parser_handles_rename_records() {
        let parsed =
            parse_porcelain_status_path("R  docs/old.md -> docs/new.md").expect("path present");
        assert_eq!(parsed, "docs/new.md");
    }

    #[test]
    fn changed_package_detection_matches_workspace_path_prefixes() {
        let workspace = vec![
            WorkspacePackage {
                name: "site".into(),
                rel_dir: "crates/site".into(),
            },
            WorkspacePackage {
                name: "xtask".into(),
                rel_dir: "xtask".into(),
            },
        ];

        let changed = vec![
            "crates/site/src/lib.rs".to_string(),
            "xtask/src/main.rs".to_string(),
        ];
        let detected = detect_changed_packages(&changed, &workspace);
        assert!(detected.contains("site"));
        assert!(detected.contains("xtask"));
    }

    #[test]
    fn no_color_normalization_maps_numeric_true() {
        assert_eq!(normalized_no_color_value(Some("1")), Some("true"));
    }

    #[test]
    fn no_color_normalization_keeps_other_values_untouched() {
        assert_eq!(normalized_no_color_value(Some("true")), None);
        assert_eq!(normalized_no_color_value(Some("false")), None);
        assert_eq!(normalized_no_color_value(None), None);
    }

    #[test]
    fn trunk_command_detection_identifies_serve_processes() {
        assert!(looks_like_trunk_serve_command("trunk serve index.html"));
        assert!(looks_like_trunk_serve_command(
            "/usr/local/bin/trunk serve --port 8080"
        ));
        assert!(!looks_like_trunk_serve_command("cargo check --workspace"));
    }

    #[test]
    fn verify_option_parser_defaults_to_full_mode() {
        let options = parse_verify_options(Vec::new()).expect("parse");
        assert_eq!(
            options,
            VerifyOptions {
                mode: VerifyMode::Full,
                desktop_mode: VerifyFastDesktopMode::Auto,
                explicit_mode: false,
                explicit_desktop_mode: false,
                profile: None,
                show_help: false
            }
        );
    }

    #[test]
    fn verify_option_parser_accepts_fast_desktop_flags() {
        let options =
            parse_verify_options(vec!["fast".into(), "--with-desktop".into()]).expect("parse");
        assert_eq!(
            options,
            VerifyOptions {
                mode: VerifyMode::Fast,
                desktop_mode: VerifyFastDesktopMode::WithDesktop,
                explicit_mode: true,
                explicit_desktop_mode: true,
                profile: None,
                show_help: false
            }
        );

        let options =
            parse_verify_options(vec!["fast".into(), "--without-desktop".into()]).expect("parse");
        assert_eq!(
            options,
            VerifyOptions {
                mode: VerifyMode::Fast,
                desktop_mode: VerifyFastDesktopMode::WithoutDesktop,
                explicit_mode: true,
                explicit_desktop_mode: true,
                profile: None,
                show_help: false
            }
        );
    }

    #[test]
    fn verify_option_parser_rejects_conflicting_desktop_flags() {
        let err = parse_verify_options(vec![
            "fast".into(),
            "--with-desktop".into(),
            "--without-desktop".into(),
        ])
        .expect_err("must reject conflicting desktop flags");
        assert!(err.contains("cannot be combined"));
    }

    #[test]
    fn verify_option_parser_supports_profiles() {
        let parsed =
            parse_verify_options(vec!["--profile".into(), "dev".into()]).expect("parse options");
        let options =
            resolve_verify_options_from_profile(parsed, &test_profiles()).expect("resolve profile");
        assert_eq!(options.mode, VerifyMode::Fast);
        assert_eq!(options.desktop_mode, VerifyFastDesktopMode::Auto);
        assert_eq!(options.profile.as_deref(), Some("dev"));
    }

    #[test]
    fn doctor_option_parser_accepts_fix() {
        let options = parse_doctor_options(vec!["--fix".to_string()]).expect("parse");
        assert_eq!(
            options,
            DoctorOptions {
                fix: true,
                show_help: false
            }
        );
    }

    #[test]
    fn desktop_trigger_detection_matches_expected_paths() {
        assert!(looks_like_desktop_host_change(
            "crates/desktop_tauri/src/main.rs"
        ));
        assert!(looks_like_desktop_host_change(
            "crates/platform_host/src/lib.rs"
        ));
        assert!(looks_like_desktop_host_change(".cargo/config.toml"));
        assert!(!looks_like_desktop_host_change(
            "crates/site/src/web_app.rs"
        ));
    }

    #[test]
    fn verify_fast_auto_detection_skips_when_no_desktop_trigger_paths_changed() {
        let decision =
            infer_verify_fast_desktop_decision(vec!["crates/site/src/web_app.rs".to_string()]);
        assert!(!decision.include_desktop);
        assert_eq!(decision.reason, "no desktop host trigger paths changed");
    }

    #[test]
    fn verify_fast_auto_detection_includes_when_desktop_trigger_paths_changed() {
        let decision = infer_verify_fast_desktop_decision(vec![
            "crates/site/src/web_app.rs".to_string(),
            "crates/desktop_tauri/src/main.rs".to_string(),
        ]);
        assert!(decision.include_desktop);
        assert!(
            decision
                .reason
                .contains("desktop-relevant changes detected"),
            "unexpected reason: {}",
            decision.reason
        );
    }
}
