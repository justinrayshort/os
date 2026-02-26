//! Workspace maintenance and developer workflow commands (`cargo xtask`).
//!
//! The `xtask` binary wraps common prototype, verification, and environment setup commands so the
//! repository can expose stable entrypoints through Cargo aliases.

use std::env;
use std::fs::{self, OpenOptions};
use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
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
           verify [fast|full]  Run scripts/ci/verify.sh (default: full)\n"
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

fn verify(root: &Path, args: Vec<String>) -> Result<(), String> {
    let mode = args.first().map(String::as_str).unwrap_or("full");
    match mode {
        "fast" | "full" => run_owned(root, "./scripts/ci/verify.sh", vec![mode.to_string()]),
        _ => Err(format!(
            "invalid verify mode `{mode}` (expected `fast` or `full`)"
        )),
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
}
