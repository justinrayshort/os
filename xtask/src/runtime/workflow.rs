//! Workflow recording, stage timing, and structured run artifacts.

use crate::runtime::artifacts::ArtifactManager;
use crate::runtime::error::{XtaskError, XtaskResult};
use serde::Serialize;
use std::env;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
struct AutomationStageRecord {
    name: String,
    started_unix_ms: u64,
    duration_ms: u128,
    status: String,
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct AutomationRunManifest {
    workflow: String,
    profile: Option<String>,
    started_unix_ms: u64,
    finished_unix_ms: u64,
    duration_ms: u128,
    status: String,
    error: Option<String>,
    run_dir: String,
    command: String,
    stages: Vec<AutomationStageRecord>,
}

#[derive(Debug)]
struct AutomationRunRecorder {
    workflow: String,
    profile: Option<String>,
    started_unix_ms: u64,
    started_instant: Instant,
    run_dir: PathBuf,
    manifest_path: PathBuf,
    events_path: PathBuf,
    command: String,
    stages: Vec<AutomationStageRecord>,
}

static ACTIVE_RUN_RECORDER: OnceLock<Mutex<Option<AutomationRunRecorder>>> = OnceLock::new();

fn active_run_recorder() -> &'static Mutex<Option<AutomationRunRecorder>> {
    ACTIVE_RUN_RECORDER.get_or_init(|| Mutex::new(None))
}

/// Shared workflow recorder service.
///
/// `WorkflowRecorder` provides the stable artifact and event vocabulary for multi-stage xtask
/// workflows. Command families that want structured run artifacts should execute through
/// [`with_workflow_run`](Self::with_workflow_run) and nest stage work in
/// [`run_timed_stage`](Self::run_timed_stage).
#[derive(Clone, Debug)]
pub struct WorkflowRecorder {
    artifacts: ArtifactManager,
}

impl WorkflowRecorder {
    /// Create a recorder service.
    pub fn new(artifacts: ArtifactManager) -> Self {
        Self { artifacts }
    }

    /// Run a workflow with manifest and event recording.
    ///
    /// This method creates a run directory under `.artifacts/automation/runs/`, emits the
    /// `workflow_started` and `workflow_finished` events, and writes a manifest summarizing the
    /// run outcome.
    pub fn with_workflow_run<F>(
        &self,
        workflow: &str,
        profile: Option<String>,
        action: F,
    ) -> XtaskResult<()>
    where
        F: FnOnce() -> XtaskResult<()>,
    {
        let recorder = self.begin_workflow_run(workflow, profile)?;
        {
            let mut guard = active_run_recorder()
                .lock()
                .map_err(|_| XtaskError::io("failed to lock workflow recorder"))?;
            *guard = Some(recorder);
        }

        let result = action();
        self.finish_workflow_run(result.as_ref().err().cloned())?;
        result
    }

    fn begin_workflow_run(
        &self,
        workflow: &str,
        profile: Option<String>,
    ) -> XtaskResult<AutomationRunRecorder> {
        let started_unix_ms = unix_timestamp_millis();
        let run_id = format!("{started_unix_ms}-{workflow}");
        let run_dir = self.artifacts.automation_runs_dir().join(run_id);
        fs::create_dir_all(&run_dir).map_err(|err| {
            XtaskError::io(format!("failed to create {}: {err}", run_dir.display()))
        })?;

        let events_path = run_dir.join("events.jsonl");
        let manifest_path = run_dir.join("manifest.json");
        fs::write(&events_path, "").map_err(|err| {
            XtaskError::io(format!(
                "failed to initialize {}: {err}",
                events_path.display()
            ))
        })?;

        append_run_event(
            &events_path,
            serde_json::json!({
                "type": "workflow_started",
                "workflow": workflow,
                "profile": profile,
                "timestamp_unix_ms": started_unix_ms
            }),
        )?;

        Ok(AutomationRunRecorder {
            workflow: workflow.to_string(),
            profile,
            started_unix_ms,
            started_instant: Instant::now(),
            run_dir,
            manifest_path,
            events_path,
            command: env::args().collect::<Vec<_>>().join(" "),
            stages: Vec::new(),
        })
    }

    fn finish_workflow_run(&self, error: Option<XtaskError>) -> XtaskResult<()> {
        let mut guard = active_run_recorder()
            .lock()
            .map_err(|_| XtaskError::io("failed to lock workflow recorder"))?;
        let Some(recorder) = guard.take() else {
            return Ok(());
        };

        let finished_unix_ms = unix_timestamp_millis();
        let status = if error.is_none() { "ok" } else { "failed" }.to_string();
        let manifest = AutomationRunManifest {
            workflow: recorder.workflow.clone(),
            profile: recorder.profile.clone(),
            started_unix_ms: recorder.started_unix_ms,
            finished_unix_ms,
            duration_ms: recorder.started_instant.elapsed().as_millis(),
            status: status.clone(),
            error: error.as_ref().map(ToString::to_string),
            run_dir: recorder.run_dir.display().to_string(),
            command: recorder.command.clone(),
            stages: recorder.stages,
        };

        append_run_event(
            &recorder.events_path,
            serde_json::json!({
                "type": "workflow_finished",
                "workflow": recorder.workflow,
                "timestamp_unix_ms": finished_unix_ms,
                "status": status,
                "error": error.as_ref().map(ToString::to_string)
            }),
        )?;

        let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|err| {
            XtaskError::io(format!("failed to serialize automation manifest: {err}"))
        })?;
        fs::write(&recorder.manifest_path, manifest_json).map_err(|err| {
            XtaskError::io(format!(
                "failed to write {}: {err}",
                recorder.manifest_path.display()
            ))
        })?;
        println!(
            "    automation run artifact: {}",
            recorder.manifest_path.display()
        );

        Ok(())
    }

    /// Record a stage with timing and structured events.
    ///
    /// The stage result is propagated to the caller unchanged after the corresponding
    /// `stage_started` and `stage_finished` events are recorded.
    pub fn run_timed_stage<F>(&self, message: &str, action: F) -> XtaskResult<()>
    where
        F: FnOnce() -> XtaskResult<()>,
    {
        println!("\n==> {message}");
        let started = Instant::now();
        let started_unix_ms = unix_timestamp_millis();
        append_active_run_event(serde_json::json!({
            "type": "stage_started",
            "name": message,
            "timestamp_unix_ms": started_unix_ms
        }))?;

        match action() {
            Ok(()) => {
                let elapsed = started.elapsed();
                let stage = AutomationStageRecord {
                    name: message.to_string(),
                    started_unix_ms,
                    duration_ms: elapsed.as_millis(),
                    status: "ok".to_string(),
                    error: None,
                };
                record_stage_event(stage, unix_timestamp_millis())?;
                println!("    done in {}", format_duration(elapsed));
                Ok(())
            }
            Err(err) => {
                let elapsed = started.elapsed();
                let stage = AutomationStageRecord {
                    name: message.to_string(),
                    started_unix_ms,
                    duration_ms: elapsed.as_millis(),
                    status: "failed".to_string(),
                    error: Some(err.to_string()),
                };
                record_stage_event(stage, unix_timestamp_millis())?;
                println!("    failed in {}", format_duration(elapsed));
                Err(err)
            }
        }
    }

    /// Print a warning using the shared workflow output style.
    pub fn warn(&self, message: &str) {
        println!("\n[warn] {message}");
    }
}

fn append_run_event(path: &Path, event: serde_json::Value) -> XtaskResult<()> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| XtaskError::io(format!("failed to open {}: {err}", path.display())))?;
    let line = serde_json::to_string(&event)
        .map_err(|err| XtaskError::io(format!("failed to serialize run event: {err}")))?;
    use std::io::Write as _;
    writeln!(&mut file, "{line}")
        .map_err(|err| XtaskError::io(format!("failed to append {}: {err}", path.display())))
}

fn append_active_run_event(event: serde_json::Value) -> XtaskResult<()> {
    let guard = active_run_recorder()
        .lock()
        .map_err(|_| XtaskError::io("failed to lock workflow recorder"))?;
    let Some(recorder) = guard.as_ref() else {
        return Ok(());
    };
    append_run_event(&recorder.events_path, event)
}

fn record_stage_event(stage: AutomationStageRecord, end_timestamp_unix_ms: u64) -> XtaskResult<()> {
    let mut guard = active_run_recorder()
        .lock()
        .map_err(|_| XtaskError::io("failed to lock workflow recorder"))?;
    let Some(recorder) = guard.as_mut() else {
        return Ok(());
    };
    append_run_event(
        &recorder.events_path,
        serde_json::json!({
            "type": "stage_finished",
            "name": stage.name,
            "started_unix_ms": stage.started_unix_ms,
            "finished_unix_ms": end_timestamp_unix_ms,
            "duration_ms": stage.duration_ms,
            "status": stage.status,
            "error": stage.error
        }),
    )?;
    recorder.stages.push(stage);
    Ok(())
}

/// Format a duration for human-readable terminal output.
pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let millis = duration.subsec_millis();
    if secs >= 60 {
        let minutes = secs / 60;
        let rem_secs = secs % 60;
        format!("{minutes}m {rem_secs}.{millis:03}s")
    } else {
        format!("{secs}.{millis:03}s")
    }
}

/// Return the current unix timestamp in seconds.
pub fn unix_timestamp_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Return the current unix timestamp in milliseconds.
pub fn unix_timestamp_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn workflow_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn unique_temp_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "xtask-workflow-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    #[test]
    fn workflow_run_writes_manifest_and_events() {
        let _guard = workflow_test_lock().lock().expect("lock workflow test");
        let root = unique_temp_root();
        let artifacts = ArtifactManager::new(root.clone());
        let workflow = WorkflowRecorder::new(artifacts.clone());

        workflow
            .with_workflow_run("runtime-test", Some("local".into()), || {
                workflow.run_timed_stage("example stage", || Ok(()))
            })
            .expect("workflow run");

        let run_root = artifacts.automation_runs_dir();
        let entries = fs::read_dir(&run_root)
            .expect("read run dir")
            .map(|entry| entry.expect("entry").path())
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);

        let manifest_path = entries[0].join("manifest.json");
        let events_path = entries[0].join("events.jsonl");
        let manifest = fs::read_to_string(&manifest_path).expect("manifest");
        let events = fs::read_to_string(&events_path).expect("events");

        assert!(manifest.contains("\"workflow\": \"runtime-test\""));
        assert!(manifest.contains("\"profile\": \"local\""));
        assert!(manifest.contains("\"status\": \"ok\""));
        assert!(events.contains("\"type\":\"workflow_started\""));
        assert!(events.contains("\"type\":\"stage_started\""));
        assert!(events.contains("\"type\":\"stage_finished\""));
        assert!(events.contains("\"type\":\"workflow_finished\""));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn failed_stage_marks_manifest_as_failed() {
        let _guard = workflow_test_lock().lock().expect("lock workflow test");
        let root = unique_temp_root();
        let artifacts = ArtifactManager::new(root.clone());
        let workflow = WorkflowRecorder::new(artifacts.clone());

        let result = workflow.with_workflow_run("runtime-test", None, || {
            workflow.run_timed_stage("failing stage", || {
                Err(XtaskError::validation("expected failure"))
            })
        });
        assert!(result.is_err());

        let run_root = artifacts.automation_runs_dir();
        let entries = fs::read_dir(&run_root)
            .expect("read run dir")
            .map(|entry| entry.expect("entry").path())
            .collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);

        let manifest = fs::read_to_string(entries[0].join("manifest.json")).expect("manifest");
        assert!(manifest.contains("\"status\": \"failed\""));
        assert!(manifest.contains("expected failure"));

        let _ = fs::remove_dir_all(root);
    }
}
