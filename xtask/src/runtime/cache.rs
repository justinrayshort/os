//! Shared `sccache` configuration and validation helpers.
//!
//! This module defines the repository-owned compiler-cache contract used by xtask workflows and
//! direct Cargo invocations inside the workspace. The contract is intentionally strict:
//! `sccache` must be present, the workspace-local cache directory must be healthy, and Cargo child
//! processes launched by xtask must inherit the canonical cache environment.

use crate::runtime::error::{XtaskError, XtaskResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const SCCACHE_BIN: &str = "sccache";
const SCCACHE_DIR_RELATIVE: &str = ".artifacts/sccache";
const SCCACHE_CACHE_SIZE: &str = "20G";

/// Canonical workspace-scoped `sccache` configuration.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SccacheConfig {
    /// Compiler wrapper executable passed to Cargo.
    pub wrapper: String,
    /// Workspace-scoped cache directory.
    pub dir: PathBuf,
    /// Maximum cache size passed to `sccache`.
    pub cache_size: String,
    /// Cache backend description. This workspace uses local disk only.
    pub backend: &'static str,
}

impl SccacheConfig {
    /// Resolve the canonical cache configuration for a workspace root.
    pub fn for_workspace(root: &Path) -> Self {
        Self {
            wrapper: SCCACHE_BIN.to_string(),
            dir: root.join(SCCACHE_DIR_RELATIVE),
            cache_size: SCCACHE_CACHE_SIZE.to_string(),
            backend: "local-disk",
        }
    }

    /// Return canonical environment overrides for Cargo child processes.
    pub fn env_pairs(&self) -> Vec<(&'static str, String)> {
        vec![
            ("RUSTC_WRAPPER", self.wrapper.clone()),
            ("SCCACHE_DIR", self.dir.display().to_string()),
            ("SCCACHE_CACHE_SIZE", self.cache_size.clone()),
        ]
    }
}

/// Health summary for the canonical `sccache` contract.
#[derive(Clone, Debug, Serialize)]
pub struct SccacheStatus {
    /// Canonical workspace configuration.
    pub config: SccacheConfig,
    /// Resolved `sccache` binary path, when discoverable.
    pub binary_path: String,
    /// `sccache` client version string.
    pub version: String,
    /// Whether the cache directory already existed before validation.
    pub cache_dir_preexisting: bool,
    /// Parsed statistics snapshot confirming backend health.
    pub stats: SccacheStatsReport,
}

/// Parsed `sccache --show-stats --stats-format json` output.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SccacheStatsReport {
    /// Statistics bucket reported by `sccache`.
    pub stats: SccacheCounters,
    /// Human-readable backend location.
    pub cache_location: String,
    /// Actual on-disk size when reported.
    pub cache_size: Option<u64>,
    /// Configured maximum cache size in bytes.
    pub max_cache_size: u64,
    /// `sccache` client version.
    pub version: String,
}

/// Subset of `sccache` counters used by performance reporting.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SccacheCounters {
    /// Total compile requests observed.
    pub compile_requests: u64,
    /// Requests actually executed by the compiler.
    pub requests_executed: u64,
    /// Cache hits broken down by language/compiler family.
    pub cache_hits: SccacheCounterMap,
    /// Cache misses broken down by language/compiler family.
    pub cache_misses: SccacheCounterMap,
    /// Cache writes recorded.
    pub cache_writes: u64,
    /// Compiler invocations performed.
    pub compilations: u64,
    /// Requests that were not cacheable.
    pub requests_not_cacheable: u64,
    /// Non-cacheable reasons.
    #[serde(default)]
    pub not_cached: BTreeMap<String, u64>,
}

impl SccacheCounters {
    /// Return the total cache-hit count across all language buckets.
    pub fn total_hits(&self) -> u64 {
        self.cache_hits.total()
    }

    /// Return the total cache-miss count across all language buckets.
    pub fn total_misses(&self) -> u64 {
        self.cache_misses.total()
    }
}

/// Counter map used by `sccache` JSON stats output.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SccacheCounterMap {
    /// Primary counters by compiler family.
    #[serde(default)]
    pub counts: BTreeMap<String, u64>,
}

impl SccacheCounterMap {
    /// Sum all counters.
    pub fn total(&self) -> u64 {
        self.counts.values().copied().sum()
    }
}

/// Simple difference between two `sccache` stats snapshots.
#[derive(Clone, Debug, Default, Serialize)]
pub struct SccacheStatsDelta {
    /// Added compile requests.
    pub compile_requests: u64,
    /// Added compiler executions.
    pub requests_executed: u64,
    /// Added cache hits.
    pub cache_hits: u64,
    /// Added cache misses.
    pub cache_misses: u64,
    /// Added cache writes.
    pub cache_writes: u64,
    /// Added compilation count.
    pub compilations: u64,
}

impl SccacheStatsDelta {
    /// Build a delta from two snapshots, saturating at zero.
    pub fn between(before: &SccacheStatsReport, after: &SccacheStatsReport) -> Self {
        Self {
            compile_requests: after
                .stats
                .compile_requests
                .saturating_sub(before.stats.compile_requests),
            requests_executed: after
                .stats
                .requests_executed
                .saturating_sub(before.stats.requests_executed),
            cache_hits: after
                .stats
                .total_hits()
                .saturating_sub(before.stats.total_hits()),
            cache_misses: after
                .stats
                .total_misses()
                .saturating_sub(before.stats.total_misses()),
            cache_writes: after
                .stats
                .cache_writes
                .saturating_sub(before.stats.cache_writes),
            compilations: after
                .stats
                .compilations
                .saturating_sub(before.stats.compilations),
        }
    }
}

/// Validate and manage the workspace compiler-cache contract.
#[derive(Clone, Debug)]
pub struct CacheService {
    root: PathBuf,
}

impl CacheService {
    /// Create a cache service rooted at the workspace.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Return the workspace root used by this service.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the canonical cache configuration.
    pub fn config(&self) -> SccacheConfig {
        SccacheConfig::for_workspace(&self.root)
    }

    /// Validate the cache contract, optionally creating the cache directory when missing.
    pub fn validate(&self, create_dir: bool) -> XtaskResult<SccacheStatus> {
        validate_sccache_config(self.root(), create_dir)
    }

    /// Start the cache server and print the effective contract, creating the directory if needed.
    pub fn bootstrap(&self) -> XtaskResult<SccacheStatus> {
        self.validate(true)
    }

    /// Return a parsed stats snapshot for the canonical cache configuration.
    pub fn stats(&self) -> XtaskResult<SccacheStatsReport> {
        fetch_sccache_stats(&self.config())
    }

    /// Zero `sccache` counters for repeatable benchmark runs.
    pub fn zero_stats(&self) -> XtaskResult<()> {
        let config = self.config();
        ensure_sccache_binary()?;
        let status = Command::new(SCCACHE_BIN)
            .args(["--zero-stats"])
            .env("SCCACHE_DIR", &config.dir)
            .env("SCCACHE_CACHE_SIZE", &config.cache_size)
            .status()
            .map_err(|err| {
                XtaskError::process_launch(format!("failed to start `sccache --zero-stats`: {err}"))
            })?;
        if status.success() {
            Ok(())
        } else {
            Err(XtaskError::process_exit(format!(
                "`sccache --zero-stats` exited with status {status}"
            )))
        }
    }

    /// Apply canonical cache environment variables to a child command.
    pub fn apply_env(&self, cmd: &mut Command) {
        for (key, value) in self.config().env_pairs() {
            cmd.env(key, value);
        }
    }
}

/// Validate the canonical workspace cache configuration.
pub fn validate_sccache_config(root: &Path, create_dir: bool) -> XtaskResult<SccacheStatus> {
    let config = SccacheConfig::for_workspace(root);
    let binary_path = resolve_sccache_binary_path()?;
    let version = fetch_sccache_version()?;
    let cache_dir_preexisting = config.dir.exists();
    ensure_cache_dir(&config, create_dir)?;
    start_sccache_server(&config)?;
    let stats = fetch_sccache_stats(&config)?;
    ensure_cache_location_matches(&config, &stats)?;

    Ok(SccacheStatus {
        config,
        binary_path,
        version,
        cache_dir_preexisting,
        stats,
    })
}

fn resolve_sccache_binary_path() -> XtaskResult<String> {
    ensure_sccache_binary()?;
    let lookup_program = if cfg!(windows) { "where" } else { "which" };
    let output = Command::new(lookup_program)
        .arg(SCCACHE_BIN)
        .output()
        .map_err(|err| {
            XtaskError::process_launch(format!(
                "failed to start `{lookup_program} {SCCACHE_BIN}`: {err}"
            ))
        })?;
    if !output.status.success() {
        return Ok(SCCACHE_BIN.to_string());
    }
    let path = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or(SCCACHE_BIN)
        .trim()
        .to_string();
    Ok(path)
}

fn fetch_sccache_version() -> XtaskResult<String> {
    let output = Command::new(SCCACHE_BIN)
        .arg("--version")
        .output()
        .map_err(|err| {
            XtaskError::process_launch(format!("failed to start `sccache --version`: {err}"))
        })?;
    if !output.status.success() {
        return Err(XtaskError::process_exit(format!(
            "`sccache --version` exited with status {}",
            output.status
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn ensure_sccache_binary() -> XtaskResult<()> {
    let status = Command::new(SCCACHE_BIN)
        .arg("--version")
        .status()
        .map_err(|_| {
            XtaskError::environment("required compiler cache `sccache` is not available on PATH")
                .with_hint(
                    "install it with `cargo install sccache`, then run `cargo cache bootstrap`",
                )
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(
            XtaskError::environment("required compiler cache `sccache` is installed but not healthy")
                .with_hint("run `sccache --version` and then `cargo cache doctor`"),
        )
    }
}

fn ensure_cache_dir(config: &SccacheConfig, create_dir: bool) -> XtaskResult<()> {
    if config.dir.exists() {
        if !config.dir.is_dir() {
            return Err(
                XtaskError::validation(format!(
                    "configured sccache path is not a directory: {}",
                    config.dir.display()
                ))
                .with_path(&config.dir),
            );
        }
    } else if create_dir {
        fs::create_dir_all(&config.dir).map_err(|err| {
            XtaskError::io(format!(
                "failed to create sccache directory {}: {err}",
                config.dir.display()
            ))
            .with_path(&config.dir)
        })?;
    } else {
        return Err(
            XtaskError::environment(format!(
                "configured sccache directory does not exist: {}",
                config.dir.display()
            ))
            .with_path(&config.dir)
            .with_hint("run `cargo cache bootstrap` or `cargo doctor --fix`"),
        );
    }

    let probe_path = config.dir.join(".xtask-write-test");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&probe_path)
        .map_err(|err| {
            XtaskError::io(format!(
                "failed to write to sccache directory {}: {err}",
                config.dir.display()
            ))
            .with_path(&config.dir)
        })?;
    file.write_all(b"ok").map_err(|err| {
        XtaskError::io(format!(
            "failed to validate sccache directory {}: {err}",
            config.dir.display()
        ))
        .with_path(&config.dir)
    })?;
    fs::remove_file(&probe_path).map_err(|err| {
        XtaskError::io(format!(
            "failed to remove sccache probe file {}: {err}",
            probe_path.display()
        ))
        .with_path(&probe_path)
    })?;
    Ok(())
}

fn start_sccache_server(config: &SccacheConfig) -> XtaskResult<()> {
    let status = Command::new(SCCACHE_BIN)
        .arg("--start-server")
        .env("SCCACHE_DIR", &config.dir)
        .env("SCCACHE_CACHE_SIZE", &config.cache_size)
        .status()
        .map_err(|err| {
            XtaskError::process_launch(format!("failed to start `sccache --start-server`: {err}"))
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(XtaskError::process_exit(format!(
            "`sccache --start-server` exited with status {status}"
        )))
    }
}

fn fetch_sccache_stats(config: &SccacheConfig) -> XtaskResult<SccacheStatsReport> {
    let output = Command::new(SCCACHE_BIN)
        .args(["--show-stats", "--stats-format", "json"])
        .env("SCCACHE_DIR", &config.dir)
        .env("SCCACHE_CACHE_SIZE", &config.cache_size)
        .output()
        .map_err(|err| {
            XtaskError::process_launch(format!(
                "failed to start `sccache --show-stats --stats-format json`: {err}"
            ))
        })?;
    if !output.status.success() {
        return Err(XtaskError::process_exit(format!(
            "`sccache --show-stats --stats-format json` exited with status {}",
            output.status
        )));
    }
    serde_json::from_slice(&output.stdout).map_err(|err| {
        XtaskError::validation(format!("failed to parse `sccache` JSON stats output: {err}"))
    })
}

fn ensure_cache_location_matches(
    config: &SccacheConfig,
    stats: &SccacheStatsReport,
) -> XtaskResult<()> {
    let configured = config.dir.display().to_string();
    if stats.cache_location.contains(&configured) {
        Ok(())
    } else {
        Err(XtaskError::validation(format!(
            "sccache backend mismatch: expected local disk cache rooted at {}, got `{}`",
            configured, stats.cache_location
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_config_uses_repo_local_cache_contract() {
        let config = SccacheConfig::for_workspace(Path::new("/tmp/workspace"));
        assert_eq!(config.wrapper, "sccache");
        assert_eq!(config.dir, Path::new("/tmp/workspace/.artifacts/sccache"));
        assert_eq!(config.cache_size, "20G");
        assert_eq!(config.backend, "local-disk");
    }

    #[test]
    fn stats_delta_saturates_and_sums_hit_buckets() {
        let before = SccacheStatsReport {
            stats: SccacheCounters {
                compile_requests: 10,
                requests_executed: 2,
                cache_hits: SccacheCounterMap {
                    counts: BTreeMap::from([(String::from("Rust"), 3)]),
                },
                cache_misses: SccacheCounterMap {
                    counts: BTreeMap::from([(String::from("Rust"), 4)]),
                },
                cache_writes: 1,
                compilations: 2,
                requests_not_cacheable: 0,
                not_cached: BTreeMap::new(),
            },
            cache_location: "Local disk: \"/tmp/workspace/.artifacts/sccache\"".into(),
            cache_size: None,
            max_cache_size: 20,
            version: "0.14.0".into(),
        };
        let after = SccacheStatsReport {
            stats: SccacheCounters {
                compile_requests: 15,
                requests_executed: 4,
                cache_hits: SccacheCounterMap {
                    counts: BTreeMap::from([(String::from("Rust"), 9)]),
                },
                cache_misses: SccacheCounterMap {
                    counts: BTreeMap::from([(String::from("Rust"), 5)]),
                },
                cache_writes: 2,
                compilations: 4,
                requests_not_cacheable: 1,
                not_cached: BTreeMap::new(),
            },
            cache_location: "Local disk: \"/tmp/workspace/.artifacts/sccache\"".into(),
            cache_size: None,
            max_cache_size: 20,
            version: "0.14.0".into(),
        };

        let delta = SccacheStatsDelta::between(&before, &after);
        assert_eq!(delta.compile_requests, 5);
        assert_eq!(delta.requests_executed, 2);
        assert_eq!(delta.cache_hits, 6);
        assert_eq!(delta.cache_misses, 1);
        assert_eq!(delta.cache_writes, 1);
        assert_eq!(delta.compilations, 2);
    }
}
