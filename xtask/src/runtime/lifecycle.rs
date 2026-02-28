//! Shared process-lifecycle and port-readiness helpers.

use crate::runtime::error::{XtaskError, XtaskResult};
use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::process::{Command, Stdio};
use std::time::Duration;

/// Return whether the process with the given pid currently exists.
pub fn process_exists(pid: u32) -> XtaskResult<bool> {
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
            "managed process inspection is only supported on unix hosts",
        ))
    }
}

/// Return the full command line for a pid when available.
pub fn process_command_line(pid: u32) -> XtaskResult<Option<String>> {
    #[cfg(unix)]
    {
        let output = Command::new("ps")
            .args(["-o", "command=", "-p", &pid.to_string()])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|err| {
                XtaskError::io(format!("failed to inspect pid {pid} with `ps`: {err}"))
            })?;

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
    #[cfg(not(unix))]
    {
        let _ = pid;
        Err(XtaskError::unsupported_platform(
            "managed process inspection is only supported on unix hosts",
        ))
    }
}

/// Send a terminate signal to the given pid.
pub fn terminate_pid(pid: u32) -> XtaskResult<()> {
    signal_named(pid, SignalKind::Terminate)
}

/// Send a kill signal to the given pid.
pub fn kill_pid(pid: u32) -> XtaskResult<()> {
    signal_named(pid, SignalKind::Kill)
}

/// Return whether the host/port is reachable.
pub fn port_is_open(host: &str, port: u16) -> bool {
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
            "managed process signaling is only supported on unix hosts",
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
