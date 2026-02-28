use super::config::{load_dev_server_config, DevServerConfig};
use super::web::site_dir;
use crate::runtime::context::CommandContext;
use crate::runtime::env::EnvHelper;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::runtime::fs::read_file_tail;
use crate::runtime::lifecycle::{kill_pid, port_is_open, process_command_line, terminate_pid};
use crate::runtime::workflow::unix_timestamp_secs;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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

#[derive(Clone, Debug)]
struct DevLogsOptions {
    lines: usize,
}

pub(crate) fn dev_server_foreground(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
    let config = load_dev_server_config(ctx)?;
    ctx.process().ensure_command(
        "trunk",
        "Install it with `cargo setup-web` (or `cargo install trunk`)",
    )?;

    let parsed = parse_trunk_serve_args(args, true, &config)?;
    ctx.process()
        .run_trunk(site_dir(ctx.root()), trunk_serve_args(&parsed))
}

pub(crate) fn dev_server_start(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
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

pub(crate) fn dev_server_status(ctx: &CommandContext) -> XtaskResult<()> {
    let config = load_dev_server_config(ctx)?;
    let Some(state) = read_dev_server_state(ctx, &config)? else {
        println!("managed dev server: not running (no state file)");
        return Ok(());
    };

    match inspect_managed_pid(state.pid)? {
        ManagedPidStatus::Managed => {
            let listening = port_is_open(&state.host, state.port);
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

pub(crate) fn dev_server_stop(ctx: &CommandContext, quiet_if_missing: bool) -> XtaskResult<()> {
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
    terminate_pid(state.pid)?;
    let deadline = Instant::now() + config.stop_timeout();

    while Instant::now() < deadline {
        if !crate::runtime::lifecycle::process_exists(state.pid)? {
            remove_dev_server_state(ctx, &config)?;
            println!("managed dev server stopped");
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    kill_pid(state.pid)?;
    if !crate::runtime::lifecycle::process_exists(state.pid)? {
        remove_dev_server_state(ctx, &config)?;
        println!("managed dev server stopped");
        return Ok(());
    }

    Err(XtaskError::process_exit(format!(
        "failed to stop managed dev server pid {} (still running after SIGKILL)",
        state.pid
    )))
}

pub(crate) fn dev_server_logs(ctx: &CommandContext, args: Vec<String>) -> XtaskResult<()> {
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
    let tail = read_file_tail(&log_path, options.lines)?;
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
    EnvHelper::default().apply_no_color_override(&mut cmd);

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
        if port_is_open(&state.host, state.port) {
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

fn read_log_tail(path: &Path, max_lines: usize) -> String {
    read_file_tail(path, max_lines).unwrap_or_default()
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

#[derive(Debug, Clone)]
pub(crate) enum ManagedPidStatus {
    NotRunning,
    Managed,
    Unmanaged(String),
}

pub(crate) fn inspect_managed_pid(pid: u32) -> XtaskResult<ManagedPidStatus> {
    if !crate::runtime::lifecycle::process_exists(pid)? {
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
}
