//! Workspace maintenance and developer workflow commands (`cargo xtask`).
//!
//! The `xtask` binary wraps common prototype, verification, and environment setup commands so the
//! repository can expose stable entrypoints through Cargo aliases.

mod docs;
mod perf;

use serde::Deserialize;
use std::collections::{BTreeSet, HashSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, ExitCode, Stdio};
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
           docs <subcommand>   Docs validation/audit commands (Rust-native)\n\
           perf <subcommand>   Performance benchmarks/profiling workflows\n\
           verify [fast|full]  Run standardized local verification workflow (default: full)\n"
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
        "Usage: cargo dev [serve|start|stop|status|restart|build] [args]\n\
         \n\
         Subcommands:\n\
           (default)           Start trunk dev server in foreground (defaults to --open)\n\
           serve [trunk args]  Same as default foreground mode\n\
           start [trunk args]  Start trunk dev server in background (no browser open by default)\n\
           stop                Stop the managed background dev server\n\
           status              Show managed dev server status\n\
           restart [args]      Restart the managed background dev server\n\
           build [args]        Build a dev static bundle via trunk (non-release)\n\
         \n\
         Notes:\n\
           - `cargo dev stop` only manages servers started with `cargo dev start`.\n\
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
        if process_exists(state.pid)? {
            return Err(format!(
                "managed dev server already running (pid {}). Use `cargo dev stop` or `cargo dev restart`.",
                state.pid
            ));
        }

        eprintln!(
            "warn: removing stale dev server state (pid {} not running)",
            state.pid
        );
        remove_dev_server_state(root)?;
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

    let running = process_exists(state.pid)?;
    let listening = running && is_port_open(&state.host, state.port);

    if running {
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
        return Ok(());
    }

    println!(
        "managed dev server: stale state (pid {} not running) | last url {} | log {}",
        state.pid,
        state.url(),
        state.log_path.display()
    );
    Ok(())
}

fn dev_server_stop(root: &Path, quiet_if_missing: bool) -> Result<(), String> {
    let Some(state) = read_dev_server_state(root)? else {
        if !quiet_if_missing {
            println!("managed dev server: not running (no state file)");
        }
        return Ok(());
    };

    if !process_exists(state.pid)? {
        println!(
            "managed dev server: stale state (pid {} not running); cleaning up state",
            state.pid
        );
        remove_dev_server_state(root)?;
        return Ok(());
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
           - `tauri.conf.json` hooks run Trunk against `crates/site/index.html`.\n\
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
    run_owned(&tauri_dir(root), "cargo", tauri_args)
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
    run_owned(&tauri_dir(root), "cargo", tauri_args)
}

fn tauri_check(root: &Path) -> Result<(), String> {
    run(root, "cargo", vec!["check", "-p", "desktop_tauri"])
}

fn tauri_dir(root: &Path) -> PathBuf {
    root.join("crates/desktop_tauri")
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
    )
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
    let mode = args.first().map(String::as_str).unwrap_or("full");
    match mode {
        "fast" => verify_fast(root),
        "full" => verify_full(root),
        _ => Err(format!(
            "invalid verify mode `{mode}` (expected `fast` or `full`)"
        )),
    }
}

fn verify_fast(root: &Path) -> Result<(), String> {
    let started = Instant::now();
    run_timed_stage("Rust format and default test matrix", || {
        run_cargo_default_matrix(root)
    })?;
    run_timed_stage("Rust all-features matrix", || run_rust_feature_matrix(root))?;
    run_timed_stage("Rustdoc build and doctests", || run_rustdoc_checks(root))?;
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
        run_cargo_default_matrix(root)
    })?;
    run_timed_stage("Rust all-features matrix", || run_rust_feature_matrix(root))?;
    run_timed_stage("Rustdoc build and doctests", || run_rustdoc_checks(root))?;
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

fn run_cargo_default_matrix(root: &Path) -> Result<(), String> {
    run(root, "cargo", vec!["fmt", "--all", "--", "--check"])?;
    run(root, "cargo", vec!["check", "--workspace"])?;
    run(
        root,
        "cargo",
        vec!["test", "--workspace", "--lib", "--tests"],
    )?;
    Ok(())
}

fn run_rust_feature_matrix(root: &Path) -> Result<(), String> {
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

fn run_rustdoc_checks(root: &Path) -> Result<(), String> {
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

fn warn_stage(message: &str) {
    println!("\n[warn] {message}");
}

fn run_timed_stage<F>(message: &str, action: F) -> Result<(), String>
where
    F: FnOnce() -> Result<(), String>,
{
    println!("\n==> {message}");
    let started = Instant::now();
    action()?;
    println!("    done in {}", format_duration(started.elapsed()));
    Ok(())
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
    trunk_args.extend(spec.passthrough.clone());
    trunk_args
}

fn args_specify_dist(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--dist" || arg.starts_with("--dist="))
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

    if env::var("NO_COLOR").as_deref() == Ok("1") {
        cmd.env("NO_COLOR", "true");
    }

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
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(_) => return String::new(),
    };

    let lines: Vec<&str> = contents.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
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

fn run_trunk(cwd: PathBuf, args: Vec<String>) -> Result<(), String> {
    print_command("trunk", &args);
    let mut cmd = Command::new("trunk");
    cmd.current_dir(cwd).args(&args);

    // Some environments export NO_COLOR=1, but trunk expects "true"/"false".
    if env::var("NO_COLOR").as_deref() == Ok("1") {
        cmd.env("NO_COLOR", "true");
    }

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
    fn dist_detection_handles_split_and_inline_forms() {
        assert!(args_specify_dist(&["--dist".into(), "x".into()]));
        assert!(args_specify_dist(&["--dist=target/custom".into()]));
        assert!(!args_specify_dist(&["--release".into()]));
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
}
