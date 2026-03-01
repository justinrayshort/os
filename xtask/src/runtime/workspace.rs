//! Shared workspace-state inspection helpers.

use crate::runtime::error::{XtaskError, XtaskResult};
use serde::Deserialize;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

/// Shared workspace package metadata used by changed-scope workflows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspacePackage {
    /// Cargo package name.
    pub name: String,
    /// Manifest directory relative to the workspace root using posix separators.
    pub manifest_dir: String,
}

/// Workspace-scoped git/cargo metadata inspection.
#[derive(Clone, Debug)]
pub struct WorkspaceState {
    root: PathBuf,
}

impl WorkspaceState {
    /// Create a workspace-state inspector rooted at the given workspace path.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Return changed paths from `git status --porcelain`.
    pub fn changed_paths(&self) -> XtaskResult<Vec<String>> {
        self.git_changed_paths_at(&self.root)
    }

    /// Return changed paths from `git status --porcelain` for a specific git worktree.
    pub fn git_changed_paths_at(&self, repo_root: &Path) -> XtaskResult<Vec<String>> {
        let output = Command::new("git")
            .current_dir(repo_root)
            .args(["status", "--porcelain"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|err| {
                XtaskError::process_launch(format!(
                    "failed to start `git status --porcelain`: {err}"
                ))
            })?;

        if !output.status.success() {
            return Err(XtaskError::process_exit(format!(
                "`git status --porcelain` exited with status {}",
                output.status
            )));
        }

        let mut paths = Vec::new();
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if let Some(path) = parse_porcelain_status_path(line) {
                paths.push(path);
            }
        }
        Ok(paths)
    }

    /// Return workspace package directories from `cargo metadata`.
    pub fn packages(&self) -> XtaskResult<Vec<WorkspacePackage>> {
        let output = Command::new("cargo")
            .current_dir(&self.root)
            .args(["metadata", "--format-version", "1", "--no-deps"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|err| {
                XtaskError::process_launch(format!("failed to start `cargo metadata`: {err}"))
            })?;

        if !output.status.success() {
            return Err(XtaskError::process_exit(format!(
                "`cargo metadata` exited with status {}",
                output.status
            )));
        }

        let metadata: CargoMetadata = serde_json::from_slice(&output.stdout).map_err(|err| {
            XtaskError::validation(format!("failed to parse `cargo metadata` output: {err}"))
        })?;

        let mut packages = Vec::new();
        for member in &metadata.workspace_members {
            let Some(pkg) = metadata.packages.iter().find(|pkg| &pkg.id == member) else {
                continue;
            };
            let manifest_dir = Path::new(&pkg.manifest_path)
                .parent()
                .map(path_to_posix)
                .unwrap_or_default();
            packages.push(WorkspacePackage {
                name: pkg.name.clone(),
                manifest_dir,
            });
        }
        Ok(packages)
    }

    /// Return the current `HEAD` commit SHA, or `"unavailable"` when not resolvable.
    pub fn git_head_sha(&self) -> String {
        self.git_head_sha_at(&self.root)
    }

    /// Return the current `HEAD` commit SHA for a specific git worktree, or `"unavailable"`.
    pub fn git_head_sha_at(&self, repo_root: &Path) -> String {
        let Ok(output) = Command::new("git")
            .current_dir(repo_root)
            .args(["rev-parse", "HEAD"])
            .output()
        else {
            return "unavailable".into();
        };
        if !output.status.success() {
            return "unavailable".into();
        }
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Return the current branch name for a specific git worktree, or `None` when detached.
    pub fn git_current_branch_at(&self, repo_root: &Path) -> Option<String> {
        let Ok(output) = Command::new("git")
            .current_dir(repo_root)
            .args(["symbolic-ref", "--short", "-q", "HEAD"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
        else {
            return None;
        };
        if !output.status.success() {
            return None;
        }
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch.is_empty() {
            None
        } else {
            Some(branch)
        }
    }

    /// Return the upstream branch name for a specific git worktree, or `None` when unavailable.
    pub fn git_upstream_branch_at(&self, repo_root: &Path) -> Option<String> {
        let Ok(output) = Command::new("git")
            .current_dir(repo_root)
            .args([
                "rev-parse",
                "--abbrev-ref",
                "--symbolic-full-name",
                "@{upstream}",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
        else {
            return None;
        };
        if !output.status.success() {
            return None;
        }
        let upstream = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if upstream.is_empty() {
            None
        } else {
            Some(upstream)
        }
    }

    /// Return the configured remote URL for a specific git worktree, or `None` when unavailable.
    pub fn git_remote_url_at(&self, repo_root: &Path, remote: &str) -> Option<String> {
        let Ok(output) = Command::new("git")
            .current_dir(repo_root)
            .args(["remote", "get-url", remote])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
        else {
            return None;
        };
        if !output.status.success() {
            return None;
        }
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if url.is_empty() {
            None
        } else {
            Some(url)
        }
    }

    /// Return `(ahead, behind)` counts against the configured upstream, or `None` when unknown.
    pub fn git_ahead_behind_at(&self, repo_root: &Path) -> Option<(u32, u32)> {
        let Ok(output) = Command::new("git")
            .current_dir(repo_root)
            .args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()
        else {
            return None;
        };
        if !output.status.success() {
            return None;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut parts = text.split_whitespace();
        let ahead = parts.next()?.parse::<u32>().ok()?;
        let behind = parts.next()?.parse::<u32>().ok()?;
        Some((ahead, behind))
    }
}

#[derive(Clone, Debug, Deserialize)]
struct CargoMetadata {
    packages: Vec<CargoMetadataPackage>,
    workspace_members: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct CargoMetadataPackage {
    id: String,
    name: String,
    manifest_path: String,
}

pub(crate) fn parse_porcelain_status_path(line: &str) -> Option<String> {
    if line.len() < 4 {
        return None;
    }

    let raw = line[3..].trim();
    if let Some((_, new)) = raw.split_once(" -> ") {
        Some(new.trim().to_string())
    } else if raw.is_empty() {
        None
    } else {
        Some(raw.to_string())
    }
}

fn path_to_posix(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_test_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "xtask-workspace-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    #[test]
    fn porcelain_parser_handles_rename_records() {
        assert_eq!(
            parse_porcelain_status_path("R  old/path -> new/path"),
            Some("new/path".into())
        );
    }

    #[test]
    fn git_head_sha_is_unavailable_outside_git_repo() {
        let root = unique_test_root();
        std::fs::create_dir_all(&root).expect("create temp root");
        let state = WorkspaceState::new(root.clone());
        assert_eq!(state.git_head_sha(), "unavailable");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn git_current_branch_is_none_outside_git_repo() {
        let root = unique_test_root();
        std::fs::create_dir_all(&root).expect("create temp root");
        let state = WorkspaceState::new(root.clone());
        assert_eq!(state.git_current_branch_at(&root), None);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn git_remote_url_is_none_outside_git_repo() {
        let root = unique_test_root();
        std::fs::create_dir_all(&root).expect("create temp root");
        let state = WorkspaceState::new(root.clone());
        assert_eq!(state.git_remote_url_at(&root, "origin"), None);
        let _ = std::fs::remove_dir_all(root);
    }
}
