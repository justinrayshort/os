//! Shared background HTTP server helpers for xtask workflows.

use crate::runtime::env::EnvHelper;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::runtime::fs::read_file_tail;
use crate::runtime::lifecycle::{kill_pid, port_is_open, terminate_pid};
use crate::runtime::process::ProcessRunner;
use std::fs::OpenOptions;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Generic background HTTP server launch specification.
///
/// This abstraction is intentionally small: command domains provide the concrete process, working
/// directory, and arguments, while the runtime owns port allocation, startup polling, log capture,
/// and shutdown behavior.
#[derive(Clone, Debug)]
pub struct BackgroundHttpServerSpec {
    /// Executable name.
    pub program: String,
    /// Command arguments.
    pub args: Vec<String>,
    /// Working directory for the child process.
    pub cwd: PathBuf,
    /// Bind host used by the child process.
    pub bind_host: String,
    /// Public host used for reachability checks and URLs.
    pub public_host: String,
    /// Bound port used by the child process.
    pub port: u16,
    /// Log file path for stdout/stderr capture.
    pub log_path: PathBuf,
    /// Maximum time to wait for the server to accept connections.
    pub startup_timeout: Duration,
    /// Maximum time to wait for graceful shutdown before SIGKILL.
    pub shutdown_timeout: Duration,
}

/// Handle for a runtime-managed background HTTP server process.
#[derive(Debug)]
pub struct BackgroundHttpServerHandle {
    child: Child,
    host: String,
    port: u16,
    log_path: PathBuf,
    shutdown_timeout: Duration,
}

impl BackgroundHttpServerHandle {
    /// Return the child process id.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    /// Return the public base URL.
    pub fn base_url(&self) -> String {
        format!("http://{}:{}/", self.host, self.port)
    }
}

/// Normalize a bind host into a URL-safe public host.
pub fn normalize_host_for_url(host: &str) -> String {
    match host {
        "0.0.0.0" => "127.0.0.1".to_string(),
        "::" => "[::1]".to_string(),
        other => other.to_string(),
    }
}

/// Allocate a local ephemeral HTTP port for a future background server.
pub fn allocate_local_http_port(bind_host: &str) -> XtaskResult<u16> {
    let bind_addr = if bind_host == "::" {
        "[::1]:0"
    } else {
        "127.0.0.1:0"
    };
    let listener = TcpListener::bind(bind_addr).map_err(|err| {
        XtaskError::io(format!(
            "failed to allocate an ephemeral HTTP port on {bind_addr}: {err}"
        ))
    })?;
    let port = listener
        .local_addr()
        .map_err(|err| XtaskError::io(format!("failed to inspect allocated port: {err}")))?
        .port();
    drop(listener);
    Ok(port)
}

/// Start a background HTTP server and wait for it to become reachable.
pub fn start_background_http_server(
    process: &ProcessRunner,
    spec: &BackgroundHttpServerSpec,
) -> XtaskResult<BackgroundHttpServerHandle> {
    let log = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&spec.log_path)
        .map_err(|err| {
            XtaskError::io(format!("failed to open {}: {err}", spec.log_path.display()))
        })?;
    let log_out = log
        .try_clone()
        .map_err(|err| XtaskError::io(format!("failed to clone log handle: {err}")))?;

    process.print_command(&spec.program, &spec.args);

    let mut cmd = Command::new(&spec.program);
    cmd.current_dir(&spec.cwd)
        .args(&spec.args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_out))
        .stderr(Stdio::from(log));
    EnvHelper.apply_no_color_override(&mut cmd);

    let child = cmd.spawn().map_err(|err| {
        XtaskError::process_launch(format!("failed to start `{}`: {err}", spec.program))
    })?;

    let mut handle = BackgroundHttpServerHandle {
        child,
        host: spec.public_host.clone(),
        port: spec.port,
        log_path: spec.log_path.clone(),
        shutdown_timeout: spec.shutdown_timeout,
    };
    wait_for_background_http_server_startup(
        &mut handle.child,
        &handle.log_path,
        &handle.host,
        handle.port,
        spec.startup_timeout,
    )?;
    Ok(handle)
}

/// Stop a background HTTP server and reap the child process.
pub fn stop_background_http_server(handle: &mut BackgroundHttpServerHandle) -> XtaskResult<()> {
    let pid = handle.pid();
    terminate_pid(pid)?;
    let deadline = Instant::now() + handle.shutdown_timeout;

    while Instant::now() < deadline {
        if let Some(status) = handle.child.try_wait().map_err(|err| {
            XtaskError::process_exit(format!(
                "failed while waiting for background HTTP server shutdown: {err}"
            ))
        })? {
            if status.success() || !port_is_open(&handle.host, handle.port) {
                return Ok(());
            }
            return Err(XtaskError::process_exit(format!(
                "background HTTP server exited with status {status}"
            )));
        }

        if !port_is_open(&handle.host, handle.port) {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    kill_pid(pid)?;
    let status = handle.child.wait().map_err(|err| {
        XtaskError::process_exit(format!(
            "failed to reap background HTTP server process {pid}: {err}"
        ))
    })?;
    if status.success() || !port_is_open(&handle.host, handle.port) {
        Ok(())
    } else {
        Err(XtaskError::process_exit(format!(
            "failed to stop background HTTP server pid {pid} cleanly (status {status})"
        )))
    }
}

fn wait_for_background_http_server_startup(
    child: &mut Child,
    log_path: &Path,
    host: &str,
    port: u16,
    timeout: Duration,
) -> XtaskResult<()> {
    let deadline = Instant::now() + timeout;

    loop {
        if port_is_open(host, port) {
            return Ok(());
        }

        if let Some(status) = child.try_wait().map_err(|err| {
            XtaskError::process_exit(format!(
                "failed while checking background HTTP server startup: {err}"
            ))
        })? {
            let mut msg =
                format!("background HTTP server exited during startup with status {status}");
            let tail = read_file_tail(log_path, 20).unwrap_or_default();
            if !tail.is_empty() {
                msg.push_str(&format!("\nlog tail ({}):\n{}", log_path.display(), tail));
            }
            return Err(XtaskError::process_exit(msg));
        }

        if Instant::now() >= deadline {
            let tail = read_file_tail(log_path, 20).unwrap_or_default();
            return Err(XtaskError::process_exit(format!(
                "background HTTP server did not become reachable at {} within {:?}\nlog tail ({}):\n{}",
                format_args!("http://{host}:{port}/"),
                timeout,
                log_path.display(),
                tail
            )));
        }

        thread::sleep(Duration::from_millis(200));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_host_for_url_rewrites_wildcards() {
        assert_eq!(normalize_host_for_url("0.0.0.0"), "127.0.0.1");
        assert_eq!(normalize_host_for_url("::"), "[::1]");
        assert_eq!(normalize_host_for_url("127.0.0.1"), "127.0.0.1");
    }

    #[test]
    fn allocate_local_http_port_returns_ephemeral_port() {
        let port = allocate_local_http_port("127.0.0.1").expect("allocate port");
        assert!(port > 0);
    }
}
