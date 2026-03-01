//! Artifact path management for xtask workflows.

use crate::runtime::error::{XtaskError, XtaskResult};
use std::fs;
use std::path::{Path, PathBuf};

const AUTOMATION_RUNS_DIR: &str = ".artifacts/automation/runs";
const DOCS_AUDIT_REPORT: &str = ".artifacts/docs-audit.json";
const E2E_RUNS_DIR: &str = ".artifacts/e2e/runs";
const E2E_BASELINES_DIR: &str = "tools/e2e/baselines";
const E2E_MANIFEST_RELATIVE: &str = "reports/ui-feedback-manifest.json";

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

    /// Return the standard E2E run root used by Cargo-managed browser automation workflows.
    pub fn e2e_runs_dir(&self) -> PathBuf {
        self.path(E2E_RUNS_DIR)
    }

    /// Return the versioned baseline root used by promoted UI feedback artifacts.
    pub fn e2e_baselines_dir(&self) -> PathBuf {
        self.path(E2E_BASELINES_DIR)
    }

    /// Return the baseline directory for a promoted scenario/slice/browser/viewport tuple.
    pub fn e2e_baseline_target(
        &self,
        scenario_id: &str,
        slice_id: &str,
        browser: &str,
        viewport_id: &str,
    ) -> PathBuf {
        self.e2e_baselines_dir()
            .join(scenario_id)
            .join(slice_id)
            .join(browser)
            .join(viewport_id)
    }

    /// Resolve a run directory, manifest file, or run id to a UI feedback manifest path.
    pub fn resolve_manifest_reference(&self, reference: &str) -> XtaskResult<PathBuf> {
        let candidate = Path::new(reference);
        let path = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else if reference.contains(std::path::MAIN_SEPARATOR) || reference.ends_with(".json") {
            self.root.join(candidate)
        } else {
            self.e2e_runs_dir().join(reference)
        };

        if path.is_file() {
            return Ok(path);
        }

        let manifest = path.join(E2E_MANIFEST_RELATIVE);
        if manifest.exists() {
            Ok(manifest)
        } else {
            Err(XtaskError::io(format!(
                "unable to resolve UI feedback manifest from `{reference}`"
            )))
        }
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
        assert_eq!(manager.e2e_runs_dir(), root.join(".artifacts/e2e/runs"));
        assert_eq!(
            manager.e2e_baselines_dir(),
            root.join("tools/e2e/baselines")
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

    #[test]
    fn resolve_manifest_reference_accepts_run_directory() {
        let root = unique_temp_root();
        let manager = ArtifactManager::new(root.clone());
        let manifest =
            root.join(".artifacts/e2e/runs/123-local-dev/reports/ui-feedback-manifest.json");
        manager
            .ensure_dir(manifest.parent().expect("parent"))
            .expect("create parent");
        fs::write(&manifest, "{}").expect("write manifest");

        let resolved = manager
            .resolve_manifest_reference("123-local-dev")
            .expect("resolve");
        assert_eq!(resolved, manifest);

        let _ = fs::remove_dir_all(root);
    }
}
