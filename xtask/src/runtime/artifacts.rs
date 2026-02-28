//! Artifact path management for xtask workflows.

use crate::runtime::error::{XtaskError, XtaskResult};
use std::fs;
use std::path::{Path, PathBuf};

const AUTOMATION_RUNS_DIR: &str = ".artifacts/automation/runs";
const DOCS_AUDIT_REPORT: &str = ".artifacts/docs-audit.json";

/// Central artifact path policy for xtask.
#[derive(Clone, Debug)]
pub struct ArtifactManager {
    root: PathBuf,
}

impl ArtifactManager {
    /// Create an artifact manager rooted at the workspace.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Workspace root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve a workspace-relative artifact path.
    pub fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    /// Automation run manifest root.
    pub fn automation_runs_dir(&self) -> PathBuf {
        self.path(AUTOMATION_RUNS_DIR)
    }

    /// Standard docs audit report location.
    pub fn docs_audit_report(&self) -> PathBuf {
        self.path(DOCS_AUDIT_REPORT)
    }

    /// Ensure a directory exists.
    pub fn ensure_dir(&self, path: &Path) -> XtaskResult<()> {
        fs::create_dir_all(path)
            .map_err(|err| XtaskError::io(format!("failed to create {}: {err}", path.display())))
    }
}
