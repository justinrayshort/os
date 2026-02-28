//! Artifact path management for xtask workflows.

use crate::runtime::error::{XtaskError, XtaskResult};
use std::fs;
use std::path::{Path, PathBuf};

const AUTOMATION_RUNS_DIR: &str = ".artifacts/automation/runs";
const DOCS_AUDIT_REPORT: &str = ".artifacts/docs-audit.json";

/// Central artifact path policy for xtask.
///
/// This service keeps workspace-relative output locations consistent across workflow families.
/// Command domains should use it instead of hard-coding ad hoc joins from `ctx.root()`.
#[derive(Clone, Debug)]
pub struct ArtifactManager {
    root: PathBuf,
}

impl ArtifactManager {
    /// Create an artifact manager rooted at the workspace.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Return the workspace root path used for resolution.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve a workspace-relative artifact path.
    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Resolve a possibly-relative workspace path.
    ///
    /// Absolute paths are preserved, while relative paths are anchored to the workspace root.
    pub fn resolve_path(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }

    /// Return the standard automation run root used by [`WorkflowRecorder`](crate::runtime::workflow::WorkflowRecorder).
    pub fn automation_runs_dir(&self) -> PathBuf {
        self.path(AUTOMATION_RUNS_DIR)
    }

    /// Return the standard docs audit report location.
    pub fn docs_audit_report(&self) -> PathBuf {
        self.path(DOCS_AUDIT_REPORT)
    }

    /// Ensure a directory exists.
    ///
    /// This helper is idempotent and succeeds when the directory already exists.
    pub fn ensure_dir(&self, path: &Path) -> XtaskResult<()> {
        fs::create_dir_all(path)
            .map_err(|err| XtaskError::io(format!("failed to create {}: {err}", path.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "xtask-artifacts-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    #[test]
    fn artifact_paths_are_root_relative() {
        let root = PathBuf::from("/tmp/xtask-artifacts-root");
        let manager = ArtifactManager::new(root.clone());
        assert_eq!(
            manager.automation_runs_dir(),
            root.join(".artifacts/automation/runs")
        );
        assert_eq!(
            manager.docs_audit_report(),
            root.join(".artifacts/docs-audit.json")
        );
    }

    #[test]
    fn ensure_dir_creates_missing_directory() {
        let root = unique_temp_root();
        let manager = ArtifactManager::new(root.clone());
        let target = root.join("nested/output");
        manager.ensure_dir(&target).expect("ensure dir");
        assert!(target.is_dir());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn resolve_path_keeps_absolute_and_expands_relative() {
        let root = PathBuf::from("/tmp/xtask-artifacts-root");
        let manager = ArtifactManager::new(root.clone());
        assert_eq!(
            manager.resolve_path(Path::new("nested/output.json")),
            root.join("nested/output.json")
        );
        assert_eq!(
            manager.resolve_path(Path::new("/tmp/already-absolute.json")),
            PathBuf::from("/tmp/already-absolute.json")
        );
    }
}
