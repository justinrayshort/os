//! Shared command context passed into command families.

use crate::runtime::artifacts::ArtifactManager;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::runtime::process::ProcessRunner;
use crate::runtime::workflow::WorkflowRecorder;
use crate::runtime::workspace::WorkspaceState;
use std::path::{Path, PathBuf};

/// Shared execution context for xtask command families.
#[derive(Clone, Debug)]
pub struct CommandContext {
    root: PathBuf,
    artifacts: ArtifactManager,
    process: ProcessRunner,
    workspace: WorkspaceState,
    workflow: WorkflowRecorder,
}

impl CommandContext {
    /// Create a new command context rooted at the current workspace.
    pub fn new() -> XtaskResult<Self> {
        let root = workspace_root()?;
        let artifacts = ArtifactManager::new(root.clone());
        let process = ProcessRunner::new();
        let workspace = WorkspaceState::new(root.clone());
        let workflow = WorkflowRecorder::new(artifacts.clone());
        Ok(Self {
            root,
            artifacts,
            process,
            workspace,
            workflow,
        })
    }

    /// Workspace root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Shared artifact manager.
    pub fn artifacts(&self) -> &ArtifactManager {
        &self.artifacts
    }

    /// Shared process runner.
    pub fn process(&self) -> &ProcessRunner {
        &self.process
    }

    /// Shared workspace-state inspector.
    pub fn workspace(&self) -> &WorkspaceState {
        &self.workspace
    }

    /// Shared workflow recorder.
    pub fn workflow(&self) -> &WorkflowRecorder {
        &self.workflow
    }
}

fn workspace_root() -> XtaskResult<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| XtaskError::environment("xtask lives under workspace root"))
}
