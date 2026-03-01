//! Shared command context passed into command families.

use crate::runtime::artifacts::ArtifactManager;
use crate::runtime::cache::CacheService;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::runtime::process::ProcessRunner;
use crate::runtime::workflow::WorkflowRecorder;
use crate::runtime::workspace::WorkspaceState;
use std::path::{Path, PathBuf};

/// Shared execution context for xtask command families.
///
/// `CommandContext` is the stable extension point for command domains. It owns the workspace root
/// and wires the shared runtime services that should be reused across workflows instead of
/// reconstructed piecemeal.
///
/// Invariants:
/// - the context is rooted at the workspace containing the `xtask` crate
/// - the service handles are cheap shared wrappers and may be borrowed repeatedly
/// - command domains should prefer this context over direct construction of runtime helpers
#[derive(Clone, Debug)]
pub struct CommandContext {
    root: PathBuf,
    artifacts: ArtifactManager,
    cache: CacheService,
    process: ProcessRunner,
    workspace: WorkspaceState,
    workflow: WorkflowRecorder,
}

impl CommandContext {
    /// Create a new command context rooted at the current workspace.
    ///
    /// Returns an environment error when the `xtask` crate layout no longer resolves back to a
    /// workspace root.
    pub fn new() -> XtaskResult<Self> {
        let root = workspace_root()?;
        let artifacts = ArtifactManager::new(root.clone());
        let cache = CacheService::new(root.clone());
        let process = ProcessRunner::new();
        let workspace = WorkspaceState::new(root.clone());
        let workflow = WorkflowRecorder::new(artifacts.clone());
        Ok(Self {
            root,
            artifacts,
            cache,
            process,
            workspace,
            workflow,
        })
    }

    /// Return the resolved workspace root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return the shared artifact manager for workspace-relative output policy.
    pub fn artifacts(&self) -> &ArtifactManager {
        &self.artifacts
    }

    /// Return the shared compiler-cache service for workspace-local `sccache` policy.
    pub fn cache(&self) -> &CacheService {
        &self.cache
    }

    /// Return the shared process runner for child-process execution and simple probes.
    pub fn process(&self) -> &ProcessRunner {
        &self.process
    }

    /// Return the shared workspace-state inspector for git/cargo metadata queries.
    pub fn workspace(&self) -> &WorkspaceState {
        &self.workspace
    }

    /// Return the shared workflow recorder for stage timing and structured run artifacts.
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
