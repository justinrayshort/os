//! Development, prototype, and host-shell workflow commands.

use crate::runtime::config::ConfigLoader;
use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::runtime::workflow::unix_timestamp_secs;
use crate::XtaskCommand;
use serde::Deserialize;
use std::fs::{self, OpenOptions};
use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const DEV_SERVER_CONFIG_FILE: &str = "tools/automation/dev_server.toml";
const SITE_CARGO_FEATURE: &str = "csr";

#[derive(Clone, Debug, Deserialize)]
struct DevServerConfigFile {
    dev_server: DevServerConfig,
}

/// Typed development server configuration loaded from `tools/automation/dev_server.toml`.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct DevServerConfig {
    pub(crate) dir: String,
    pub(crate) state_file: String,
    pub(crate) log_file: String,
    pub(crate) default_host: String,
    pub(crate) default_port: u16,
    pub(crate) start_poll_secs: u64,
    pub(crate) stop_timeout_secs: u64,
}

impl DevServerConfig {
    fn validate(self) -> XtaskResult<Self> {
        if self.dir.is_empty() || self.state_file.is_empty() || self.log_file.is_empty() {
            return Err(XtaskError::config(
                "dev_server config paths must not be empty",
            ));
        }
        if self.start_poll_secs == 0 || self.stop_timeout_secs == 0 {
            return Err(XtaskError::config(
                "dev_server timeout values must be greater than zero",
            ));
        }
        Ok(self)
    }

    fn start_poll(&self) -> Duration {
        Duration::from_secs(self.start_poll_secs)
    }

    fn stop_timeout(&self) -> Duration {
        Duration::from_secs(self.stop_timeout_secs)
    }
}

fn load_dev_server_config(ctx: &CommandContext) -> XtaskResult<DevServerConfig> {
    let loader = ConfigLoader::<DevServerConfigFile>::new(ctx.root(), DEV_SERVER_CONFIG_FILE);
    loader.load()?.dev_server.validate()
}

/// `cargo setup-web`
pub struct SetupWebCommand;

impl XtaskCommand for SetupWebCommand {
    type Options = ();

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        if args.is_empty() {
            Ok(())
        } else {
            Err(XtaskError::validation(
                "`cargo setup-web` does not accept extra arguments",
            ))
        }
    }

    fn run(ctx: &CommandContext, _: Self::Options) -> XtaskResult<()> {
        ctx.process().run(
            ctx.root(),
            "rustup",
            vec!["target", "add", "wasm32-unknown-unknown"],
        )?;

        if ctx.process().command_available("trunk") {
            println!("trunk already installed");
            return Ok(());
        }

        ctx.process()
            .run(ctx.root(), "cargo", vec!["install", "trunk"])
    }
}

/// `cargo build-web`
pub struct BuildWebCommand;

impl XtaskCommand for BuildWebCommand {
    type Options = Vec<String>;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        Ok(args.to_vec())
    }

    fn run(ctx: &CommandContext, args: Self::Options) -> XtaskResult<()> {
        trunk_build(ctx, args, BuildProfile::Release)
    }
}

/// `cargo check-web`
pub struct CheckWebCommand;

impl XtaskCommand for CheckWebCommand {
    type Options = ();

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        if args.is_empty() {
            Ok(())
        } else {
            Err(XtaskError::validation(
                "`cargo check-web` does not accept extra arguments",
            ))
        }
    }

    fn run(ctx: &CommandContext, _: Self::Options) -> XtaskResult<()> {
        ctx.process().run(
            ctx.root(),
            "cargo",
            vec!["check", "-p", "site", "--features", SITE_CARGO_FEATURE],
        )?;

        if wasm_target_installed() {
            ctx.process().run(
                ctx.root(),
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
            )
        } else {
            ctx.workflow()
                .warn("wasm32-unknown-unknown target not installed; skipping wasm cargo check");
            Ok(())
        }
    }
}

/// `cargo tauri ...`
pub struct TauriCommand;

impl XtaskCommand for TauriCommand {
    type Options = Vec<String>;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        Ok(args.to_vec())
    }

    fn run(ctx: &CommandContext, args: Self::Options) -> XtaskResult<()> {
        match args.first().map(String::as_str) {
            Some("dev") => tauri_dev(ctx, args[1..].to_vec()),
            Some("build") => tauri_build(ctx, args[1..].to_vec()),
            Some("check") => tauri_check(ctx),
            Some("help" | "--help" | "-h") | None => {
                print_tauri_usage();
                Ok(())
            }
            Some(other) => Err(XtaskError::validation(format!(
                "unknown tauri subcommand: {other}"
            ))),
        }
    }
}

/// `cargo doctor`
pub struct DoctorCommand;

impl XtaskCommand for DoctorCommand {
    type Options = DoctorOptions;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        parse_doctor_options(args.to_vec())
    }

    fn run(ctx: &CommandContext, options: Self::Options) -> XtaskResult<()> {
        if options.show_help {
            print_doctor_usage();
            return Ok(());
        }
        run_doctor(ctx, options)
    }
}

/// `cargo dev ...`
pub struct DevCommand;

impl XtaskCommand for DevCommand {
    type Options = Vec<String>;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        Ok(args.to_vec())
    }

    fn run(ctx: &CommandContext, args: Self::Options) -> XtaskResult<()> {
        match args.first().map(String::as_str) {
            None => dev_server_foreground(ctx, Vec::new()),
            Some("serve") => dev_server_foreground(ctx, args[1..].to_vec()),
            Some("start") => dev_server_start(ctx, args[1..].to_vec()),
            Some("stop") => dev_server_stop(ctx, false),
            Some("status") => dev_server_status(ctx),
            Some("logs") => dev_server_logs(ctx, args[1..].to_vec()),
            Some("restart") => {
                dev_server_stop(ctx, true)?;
                dev_server_start(ctx, args[1..].to_vec())
            }
            Some("build") => trunk_build(ctx, args[1..].to_vec(), BuildProfile::Dev),
            Some("help" | "--help" | "-h") => {
                print_dev_usage();
                Ok(())
            }
            _ => dev_server_foreground(ctx, args),
        }
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
           status              Show managed background dev server status\n\
           logs [--lines N]    Print recent managed dev server log output\n\
           restart [args]      Restart the managed background dev server\n\
           build [args]        Build a dev static bundle via trunk (non-release)\n\
         \n\
         Notes:\n\
           - `cargo dev stop` only manages servers started with `cargo dev start`.\n\
           - Development serve/build defaults disable SRI; file hashing stays enabled unless explicitly overridden.\n\
           - Logs and state are stored under `.artifacts/dev-server/`.\n"
    );
}

fn print_tauri_usage() {
    eprintln!(
        "Usage: cargo xtask tauri <dev|build|check> [args]\n\
         \n\
         Subcommands:\n\
           dev [args]    Run `cargo tauri dev`\n\
           build [args]  Run `cargo tauri build`\n\
           check         Validate desktop_tauri compiles\n"
    );
}

#[derive(Clone, Debug)]
pub struct DoctorOptions {
    fix: bool,
    show_help: bool,
}

fn parse_doctor_options(args: Vec<String>) -> XtaskResult<DoctorOptions> {
    let mut options = DoctorOptions {
        fix: false,
        show_help: false,
    };

    for arg in args {
        match arg.as_str() {
            "--fix" => options.fix = true,
            "help" | "--help" | "-h" => options.show_help = true,
            other => {
                return Err(XtaskError::validation(format!(
                    "unsupported `cargo doctor` argument `{other}`"
                )))
            }
        }
    }

    Ok(options)
}

fn print_doctor_usage() {
    eprintln!(
        "Usage: cargo doctor [--fix]\n\
         \n\
         Checks local tooling and managed dev-server hygiene.\n\
         \n\
         Flags:\n\
           --fix  Remove stale managed dev server state when safe\n"
    );
}

fn run_doctor(ctx: &CommandContext, options: DoctorOptions) -> XtaskResult<()> {
    let config = load_dev_server_config(ctx)?;
    ctx.workflow()
        .with_workflow_run("doctor", None, || {
            ctx.workflow().run_timed_stage("Tooling prerequisite: trunk", || {
                if ctx.process().command_available("trunk") {
                    println!("    trunk is available");
                    Ok(())
                } else if options.fix {
                    println!("    trunk missing; running `cargo setup-web`");
                    SetupWebCommand::run(ctx, ())?;
                    if ctx.process().command_available("trunk") {
                        println!("    trunk installed");
                        Ok(())
                    } else {
                        Err(XtaskError::environment(
                            "`trunk` is not available (run `cargo setup-web`)",
                        ))
                    }
                } else {
                    Err(XtaskError::environment(
                        "`trunk` is not available (run `cargo setup-web`)",
                    ))
                }
            })?;

            ctx.workflow()
                .run_timed_stage("Tooling prerequisite: wasm target", || {
                    if wasm_target_installed() {
                        println!("    wasm32-unknown-unknown target installed");
                        Ok(())
                    } else {
                        Err(XtaskError::environment(
                            "wasm32-unknown-unknown target missing (run `cargo setup-web`)",
                        ))
                    }
                })?;

            ctx.workflow()
                .run_timed_stage("Docs prerequisite: wiki submodule", || {
                    let wiki_root = ctx.root().join("wiki");
                    if wiki_root.join(".git").exists() || wiki_root.join("Home.md").exists() {
                        println!("    wiki submodule initialized");
                        Ok(())
                    } else {
                        Err(XtaskError::environment(
                            "wiki submodule missing (run `git submodule update --init --recursive`)",
                        ))
                    }
                })?;

            ctx.workflow()
                .run_timed_stage("Dev server state hygiene", || {
                    let Some(state) = read_dev_server_state(ctx, &config)? else {
                        println!("    no managed dev server state file");
                        return Ok(());
                    };

                    match inspect_managed_pid(state.pid)? {
                        ManagedPidStatus::Managed => {
                            println!("    managed dev server state is healthy");
                            Ok(())
                        }
                        ManagedPidStatus::NotRunning | ManagedPidStatus::Unmanaged(_) if options.fix => {
                            remove_dev_server_state(ctx, &config)?;
                            println!("    removed stale managed dev server state");
                            Ok(())
                        }
                        ManagedPidStatus::NotRunning | ManagedPidStatus::Unmanaged(_) => Err(
                            XtaskError::validation(
                                "managed dev server state is stale (run `cargo dev stop` or `cargo doctor --fix`)",
                            ),
                        ),
                    }
                })?;

            println!("\n==> Doctor checks passed");
            Ok(())
        })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BuildProfile {
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
pub(crate) struct DevServerState {
    pub(crate) pid: u32,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) started_unix_secs: u64,
    pub(crate) log_path: PathBuf,
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

pub(crate) fn site_dir(root: &Path) -> PathBuf {
    root.join("crates/site")
}

fn tauri_dir(root: &Path) -> PathBuf {
    root.join("crates/desktop_tauri")
}

fn trunk_build(ctx: &CommandContext, args: Vec<String>, profile: BuildProfile) -> XtaskResult<()> {
    ctx.process().ensure_command(
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
    if profile == BuildProfile::Dev && !args_specify_no_sri(&args) {
        trunk_args.push("--no-sri=true".to_string());
    }
    trunk_args.extend(args);
    ctx.process().run_trunk(site_dir(ctx.root()), trunk_args)
}

fn tauri_dev(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    ctx.process().run_tauri_cli(
        &tauri_dir(ctx.root()),
        prepend_tauri_subcommand("tauri", "dev", args),
    )
}

fn tauri_build(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    ctx.process().run_tauri_cli(
        &tauri_dir(ctx.root()),
        prepend_tauri_subcommand("tauri", "build", args),
    )
}

fn tauri_check(ctx: &CommandContext) -> XtaskResult<()> {
    ctx.process()
        .run(ctx.root(), "cargo", vec!["check", "-p", "desktop_tauri"])
}

fn prepend_tauri_subcommand(root_cmd: &str, subcommand: &str, args: Vec<String>) -> Vec<String> {
    let mut all = vec![root_cmd.to_string(), subcommand.to_string()];
    all.extend(args);
    all
}

fn dev_server_foreground(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    let config = load_dev_server_config(ctx)?;
    ctx.process().ensure_command(
        "trunk",
        "Install it with `cargo setup-web` (or `cargo install trunk`)",
    )?;

    let parsed = parse_trunk_serve_args(args, true, &config)?;
    ctx.process()
        .run_trunk(site_dir(ctx.root()), trunk_serve_args(&parsed))
}

fn dev_server_start(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    let config = load_dev_server_config(ctx)?;
    ctx.process().ensure_command(
        "trunk",
        "Install it with `cargo setup-web` (or `cargo install trunk`)",
    )?;

    let parsed = parse_trunk_serve_args(args, false, &config)?;

    if let Some(state) = read_dev_server_state(ctx, &config)? {
        match inspect_managed_pid(state.pid)? {
            ManagedPidStatus::Managed => {
                return Err(XtaskError::validation(format!(
                    "managed dev server already running (pid {}). Use `cargo dev stop` or `cargo dev restart`.",
                    state.pid
                )));
            }
            ManagedPidStatus::NotRunning => {
                eprintln!(
                    "warn: removing stale dev server state (pid {} not running)",
                    state.pid
                );
                remove_dev_server_state(ctx, &config)?;
            }
            ManagedPidStatus::Unmanaged(command) => {
                eprintln!(
                    "warn: state pid {} belongs to a different process; cleaning managed state without signaling\n  command: {}",
                    state.pid, command
                );
                remove_dev_server_state(ctx, &config)?;
            }
        }
    }

    let log_path = dev_server_log_path(ctx, &config);
    let mut child = spawn_trunk_background(
        ctx,
        site_dir(ctx.root()),
        trunk_serve_args(&parsed),
        &log_path,
    )?;

    let state = DevServerState {
        pid: child.id(),
        host: parsed.host.clone(),
        port: parsed.port,
        started_unix_secs: unix_timestamp_secs(),
        log_path: log_path.clone(),
    };
    write_dev_server_state(ctx, &config, &state)?;

    match wait_for_startup(&mut child, &state, &config) {
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
            let _ = remove_dev_server_state(ctx, &config);
            return Err(err);
        }
    }

    Ok(())
}

fn dev_server_status(ctx: &CommandContext) -> XtaskResult<()> {
    let config = load_dev_server_config(ctx)?;
    let Some(state) = read_dev_server_state(ctx, &config)? else {
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
        }
        ManagedPidStatus::Unmanaged(command) => {
            println!(
                "managed dev server: stale state (pid {} belongs to another process) | last url {} | log {}",
                state.pid,
                state.url(),
                state.log_path.display()
            );
            println!("  command: {command}");
        }
    }

    Ok(())
}

fn dev_server_stop(ctx: &CommandContext, quiet_if_missing: bool) -> XtaskResult<()> {
    let config = load_dev_server_config(ctx)?;
    let Some(state) = read_dev_server_state(ctx, &config)? else {
        if !quiet_if_missing {
            println!("managed dev server: not running (no state file)");
        }
        return Ok(());
    };

    match inspect_managed_pid(state.pid)? {
        ManagedPidStatus::NotRunning => {
            if !quiet_if_missing {
                println!(
                    "managed dev server: stale state (pid {} not running); cleaning up state",
                    state.pid
                );
            }
            remove_dev_server_state(ctx, &config)?;
            return Ok(());
        }
        ManagedPidStatus::Unmanaged(_) => {
            if !quiet_if_missing {
                println!(
                    "managed dev server: stale state (pid {} now belongs to another process); refusing to signal and cleaning state",
                    state.pid
                );
            }
            remove_dev_server_state(ctx, &config)?;
            return Ok(());
        }
        ManagedPidStatus::Managed => {}
    }

    println!("stopping managed dev server pid {}...", state.pid);
    signal_terminate(state.pid)?;
    let deadline = Instant::now() + config.stop_timeout();

    while Instant::now() < deadline {
        if !process_exists(state.pid)? {
            remove_dev_server_state(ctx, &config)?;
            println!("managed dev server stopped");
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    signal_kill(state.pid)?;
    if !process_exists(state.pid)? {
        remove_dev_server_state(ctx, &config)?;
        println!("managed dev server stopped");
        return Ok(());
    }

    Err(XtaskError::process_exit(format!(
        "failed to stop managed dev server pid {} (still running after SIGKILL)",
        state.pid
    )))
}

#[derive(Clone, Debug)]
struct DevLogsOptions {
    lines: usize,
}

fn dev_server_logs(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    let config = load_dev_server_config(ctx)?;
    let options = parse_dev_logs_options(args)?;
    let log_path = dev_server_log_path(ctx, &config);
    if !log_path.exists() {
        return Err(XtaskError::validation(format!(
            "no managed dev server log file found at {}",
            log_path.display()
        )));
    }

    println!(
        "managed dev server log tail ({} lines) from {}",
        options.lines,
        log_path.display()
    );
    let tail = read_log_tail_with_limit(&log_path, options.lines)?;
    if tail.is_empty() {
        println!("(log is currently empty)");
    } else {
        println!("{tail}");
    }
    Ok(())
}

fn parse_dev_logs_options(args: Vec<String>) -> XtaskResult<DevLogsOptions> {
    let mut options = DevLogsOptions { lines: 80 };
    let mut i = 0usize;

    while i < args.len() {
        match args[i].as_str() {
            "--lines" => {
                let Some(value) = args.get(i + 1) else {
                    return Err(XtaskError::validation("missing value for `--lines`"));
                };
                options.lines = parse_positive_usize(value, "--lines")?;
                i += 2;
            }
            other => {
                return Err(XtaskError::validation(format!(
                    "unsupported `cargo dev logs` argument `{other}`"
                )));
            }
        }
    }

    Ok(options)
}

fn parse_positive_usize(value: &str, flag: &str) -> XtaskResult<usize> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| XtaskError::validation(format!("invalid value for `{flag}`: `{value}`")))?;
    if parsed == 0 {
        return Err(XtaskError::validation(format!(
            "`{flag}` must be greater than zero"
        )));
    }
    Ok(parsed)
}

fn parse_trunk_serve_args(
    args: Vec<String>,
    default_open: bool,
    config: &DevServerConfig,
) -> XtaskResult<TrunkServeSpec> {
    let mut passthrough = Vec::new();
    let mut open = default_open;
    let mut host = config.default_host.clone();
    let mut port = config.default_port;
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
                    return Err(XtaskError::validation(format!("missing value for `{arg}`")));
                };
                port = parse_port(value, arg)?;
                passthrough.push(arg.clone());
                passthrough.push(value.clone());
                i += 2;
                continue;
            }
            "--address" => {
                let Some(value) = args.get(i + 1) else {
                    return Err(XtaskError::validation("missing value for `--address`"));
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

fn parse_port(value: &str, flag: &str) -> XtaskResult<u16> {
    value
        .parse::<u16>()
        .map_err(|_| XtaskError::validation(format!("invalid port for `{flag}`: `{value}`")))
}

fn trunk_serve_args(spec: &TrunkServeSpec) -> Vec<String> {
    let mut trunk_args = vec!["serve".to_string(), "index.html".to_string()];
    let dist = trunk_dist_path(&spec.passthrough);
    if spec.open {
        trunk_args.push("--open".to_string());
    }
    if !args_specify_no_sri(&spec.passthrough) {
        trunk_args.push("--no-sri=true".to_string());
    }
    maybe_add_ignore(&mut trunk_args, &spec.passthrough, &dist);
    maybe_add_ignore(&mut trunk_args, &spec.passthrough, "dist");
    trunk_args.extend(spec.passthrough.clone());
    trunk_args
}

fn args_specify_dist(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--dist" || arg.starts_with("--dist="))
}

fn args_specify_no_sri(args: &[String]) -> bool {
    args.iter()
        .any(|arg| arg == "--no-sri" || arg.starts_with("--no-sri="))
}

fn trunk_dist_path(args: &[String]) -> String {
    let mut i = 0usize;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--dist" {
            if let Some(value) = args.get(i + 1) {
                return value.clone();
            }
        } else if let Some(value) = arg.strip_prefix("--dist=") {
            return value.to_string();
        }
        i += 1;
    }

    "dist".to_string()
}

fn args_specify_ignore_path(args: &[String], path: &str) -> bool {
    let mut i = 0usize;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--ignore" {
            if let Some(value) = args.get(i + 1) {
                if value == path {
                    return true;
                }
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }

        if let Some(value) = arg.strip_prefix("--ignore=") {
            if value == path {
                return true;
            }
        }

        i += 1;
    }

    false
}

fn maybe_add_ignore(trunk_args: &mut Vec<String>, passthrough: &[String], path: &str) {
    if path.is_empty() || args_specify_ignore_path(passthrough, path) {
        return;
    }
    if trunk_args
        .windows(2)
        .any(|w| w[0] == "--ignore" && w[1] == path)
    {
        return;
    }
    trunk_args.push("--ignore".to_string());
    trunk_args.push(path.to_string());
}

fn ensure_dev_server_dir(ctx: &CommandContext, config: &DevServerConfig) -> XtaskResult<PathBuf> {
    let path = ctx.root().join(&config.dir);
    fs::create_dir_all(&path)
        .map_err(|err| XtaskError::io(format!("failed to create {}: {err}", path.display())))?;
    Ok(path)
}

fn dev_server_state_path(ctx: &CommandContext, config: &DevServerConfig) -> PathBuf {
    ctx.root().join(&config.state_file)
}

fn dev_server_log_path(ctx: &CommandContext, config: &DevServerConfig) -> PathBuf {
    ctx.root().join(&config.log_file)
}

fn write_dev_server_state(
    ctx: &CommandContext,
    config: &DevServerConfig,
    state: &DevServerState,
) -> XtaskResult<()> {
    ensure_dev_server_dir(ctx, config)?;
    let body = format!(
        "pid={}\nhost={}\nport={}\nstarted_unix_secs={}\nlog_path={}\n",
        state.pid,
        state.host,
        state.port,
        state.started_unix_secs,
        state.log_path.display()
    );

    fs::write(dev_server_state_path(ctx, config), body)
        .map_err(|err| XtaskError::io(format!("failed to write dev server state: {err}")))
}

pub(crate) fn read_dev_server_state(
    ctx: &CommandContext,
    config: &DevServerConfig,
) -> XtaskResult<Option<DevServerState>> {
    let path = dev_server_state_path(ctx, config);
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&path)
        .map_err(|err| XtaskError::io(format!("failed to read {}: {err}", path.display())))?;

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
        pid: pid.ok_or_else(|| {
            XtaskError::validation(format!("invalid dev server state in {}", path.display()))
        })?,
        host: host.unwrap_or_else(|| config.default_host.clone()),
        port: port.unwrap_or(config.default_port),
        started_unix_secs: started_unix_secs.unwrap_or(0),
        log_path: log_path.unwrap_or_else(|| PathBuf::from(&config.log_file)),
    };

    Ok(Some(state))
}

pub(crate) fn remove_dev_server_state(
    ctx: &CommandContext,
    config: &DevServerConfig,
) -> XtaskResult<()> {
    let path = dev_server_state_path(ctx, config);
    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(XtaskError::io(format!(
            "failed to remove {}: {err}",
            path.display()
        ))),
    }
}

fn spawn_trunk_background(
    ctx: &CommandContext,
    cwd: PathBuf,
    args: Vec<String>,
    log_path: &Path,
) -> XtaskResult<Child> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            XtaskError::io(format!("failed to create {}: {err}", parent.display()))
        })?;
    }

    let log = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(log_path)
        .map_err(|err| XtaskError::io(format!("failed to open {}: {err}", log_path.display())))?;
    let log_out = log
        .try_clone()
        .map_err(|err| XtaskError::io(format!("failed to clone log file handle: {err}")))?;

    ctx.process().print_command("trunk", &args);
    let mut cmd = Command::new("trunk");
    cmd.current_dir(cwd)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_out))
        .stderr(Stdio::from(log));
    crate::runtime::env::EnvHelper::default().apply_no_color_override(&mut cmd);

    cmd.spawn().map_err(|err| {
        XtaskError::process_launch(format!("failed to start `trunk` in background: {err}"))
    })
}

fn wait_for_startup(
    child: &mut Child,
    state: &DevServerState,
    config: &DevServerConfig,
) -> XtaskResult<StartupState> {
    let deadline = Instant::now() + config.start_poll();

    loop {
        if is_port_open(&state.host, state.port) {
            return Ok(StartupState::Ready);
        }

        if let Some(status) = child.try_wait().map_err(|err| {
            XtaskError::process_exit(format!("failed while checking dev server startup: {err}"))
        })? {
            let mut msg = format!("managed dev server exited during startup with status {status}");
            let tail = read_log_tail(&state.log_path, 20);
            if !tail.is_empty() {
                msg.push_str(&format!(
                    "\nlog tail ({}):\n{}",
                    state.log_path.display(),
                    tail
                ));
            }
            return Err(XtaskError::process_exit(msg));
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

fn read_log_tail_with_limit(path: &Path, max_lines: usize) -> XtaskResult<String> {
    let contents = fs::read_to_string(path)
        .map_err(|err| XtaskError::io(format!("failed to read {}: {err}", path.display())))?;
    let lines: Vec<&str> = contents.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    Ok(lines[start..].join("\n"))
}

pub(crate) fn wasm_target_installed() -> bool {
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

pub(crate) fn process_exists(pid: u32) -> XtaskResult<bool> {
    #[cfg(unix)]
    {
        match signal(pid, 0) {
            Ok(()) => Ok(true),
            Err(err) => match err.raw_os_error() {
                Some(ESRCH) => Ok(false),
                Some(EPERM) => Ok(true),
                _ => Err(XtaskError::io(format!("failed to query pid {pid}: {err}"))),
            },
        }
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        Err(XtaskError::unsupported_platform(
            "managed dev server status/stop is only supported on unix hosts",
        ))
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ManagedPidStatus {
    NotRunning,
    Managed,
    Unmanaged(String),
}

pub(crate) fn inspect_managed_pid(pid: u32) -> XtaskResult<ManagedPidStatus> {
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
fn process_command_line(pid: u32) -> XtaskResult<Option<String>> {
    let output = Command::new("ps")
        .args(["-o", "command=", "-p", &pid.to_string()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|err| XtaskError::io(format!("failed to inspect pid {pid} with `ps`: {err}")))?;

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

fn signal_terminate(pid: u32) -> XtaskResult<()> {
    signal_named(pid, SignalKind::Terminate)
}

fn signal_kill(pid: u32) -> XtaskResult<()> {
    signal_named(pid, SignalKind::Kill)
}

#[derive(Clone, Copy, Debug)]
enum SignalKind {
    Terminate,
    Kill,
}

fn signal_named(pid: u32, kind: SignalKind) -> XtaskResult<()> {
    #[cfg(unix)]
    {
        let sig = match kind {
            SignalKind::Terminate => SIGTERM,
            SignalKind::Kill => SIGKILL,
        };
        match signal(pid, sig) {
            Ok(()) => Ok(()),
            Err(err) if err.raw_os_error() == Some(ESRCH) => Ok(()),
            Err(err) => Err(XtaskError::io(format!("failed to signal pid {pid}: {err}"))),
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (pid, kind);
        Err(XtaskError::unsupported_platform(
            "managed dev server stop is only supported on unix hosts",
        ))
    }
}

#[cfg(unix)]
fn signal(pid: u32, sig: i32) -> Result<(), io::Error> {
    let pid_i32 = i32::try_from(pid)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "pid out of range"))?;
    let rc = unsafe { kill(pid_i32, sig) };
    if rc == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> DevServerConfig {
        DevServerConfig {
            dir: ".artifacts/dev-server".into(),
            state_file: ".artifacts/dev-server/state.env".into(),
            log_file: ".artifacts/dev-server/trunk.log".into(),
            default_host: "127.0.0.1".into(),
            default_port: 8080,
            start_poll_secs: 8,
            stop_timeout_secs: 5,
        }
    }

    #[test]
    fn parse_foreground_dev_args_defaults_to_open() {
        let spec = parse_trunk_serve_args(Vec::new(), true, &test_config()).expect("parse");
        assert!(spec.open);
    }

    #[test]
    fn parse_managed_dev_args_tracks_port_and_address() {
        let spec = parse_trunk_serve_args(
            vec![
                "--port".into(),
                "9000".into(),
                "--address".into(),
                "0.0.0.0".into(),
            ],
            false,
            &test_config(),
        )
        .expect("parse");
        assert!(!spec.open);
        assert_eq!(spec.port, 9000);
        assert_eq!(spec.host, "0.0.0.0");
    }

    #[test]
    fn trunk_serve_args_default_to_no_sri_and_keep_filehash() {
        let spec = parse_trunk_serve_args(Vec::new(), false, &test_config()).expect("parse");
        let args = trunk_serve_args(&spec);
        assert!(args.iter().any(|arg| arg == "--no-sri=true"));
        assert!(!args.iter().any(|arg| arg == "--filehash=false"));
    }

    #[test]
    fn trunk_serve_args_respect_explicit_hash_and_sri_settings() {
        let spec = parse_trunk_serve_args(
            vec!["--filehash=false".into(), "--no-sri=false".into()],
            false,
            &test_config(),
        )
        .expect("parse");
        let args = trunk_serve_args(&spec);
        assert!(args.iter().any(|arg| arg == "--filehash=false"));
        assert!(args.iter().any(|arg| arg == "--no-sri=false"));
        assert_eq!(
            args.iter()
                .filter(|arg| arg.as_str() == "--no-sri=true")
                .count(),
            0
        );
    }

    #[test]
    fn trunk_serve_args_ignore_active_dist_directory() {
        let spec = parse_trunk_serve_args(
            vec!["--dist".into(), "target/trunk-tauri-dev".into()],
            false,
            &test_config(),
        )
        .expect("parse");
        let args = trunk_serve_args(&spec);
        assert!(args
            .windows(2)
            .any(|w| w == ["--ignore", "target/trunk-tauri-dev"]));
    }

    #[test]
    fn trunk_serve_args_do_not_duplicate_explicit_ignore_for_dist() {
        let spec = parse_trunk_serve_args(
            vec![
                "--dist=target/trunk-tauri-dev".into(),
                "--ignore=target/trunk-tauri-dev".into(),
            ],
            false,
            &test_config(),
        )
        .expect("parse");
        let args = trunk_serve_args(&spec);
        assert_eq!(
            args.iter()
                .filter(|arg| arg.as_str() == "--ignore=target/trunk-tauri-dev")
                .count(),
            1
        );
    }

    #[test]
    fn logs_option_parser_defaults_and_rejects_zero() {
        let defaults = parse_dev_logs_options(Vec::new()).expect("parse");
        assert_eq!(defaults.lines, 80);
        let err = parse_dev_logs_options(vec!["--lines".into(), "0".into()]).unwrap_err();
        assert!(err.to_string().contains("greater than zero"));
    }

    #[test]
    fn doctor_option_parser_accepts_fix() {
        let parsed = parse_doctor_options(vec!["--fix".into()]).expect("parse");
        assert!(parsed.fix);
        assert!(!parsed.show_help);
    }
}
