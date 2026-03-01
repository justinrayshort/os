//! Cargo-managed end-to-end workflow foundation.
//!
//! The browser-backed path is the canonical validation loop for deterministic neumorphic UI work.
//! This command family owns profile/scenario resolution, Playwright harness execution, schema-v2
//! manifest inspection, and baseline promotion under the Cargo-managed automation surface.

use crate::commands::dev::{load_dev_server_config, DevServerConfig};
use crate::runtime::config::ConfigLoader;
use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::runtime::serve::{
    allocate_local_http_port, normalize_host_for_url, start_background_http_server,
    stop_background_http_server, BackgroundHttpServerHandle, BackgroundHttpServerSpec,
};
use crate::runtime::workflow::unix_timestamp_millis;
use crate::XtaskCommand;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

const E2E_PROFILES_CONFIG: &str = "tools/automation/e2e_profiles.toml";
const E2E_SCENARIOS_CONFIG: &str = "tools/automation/e2e_scenarios.toml";
const E2E_NODE_PACKAGE: &str = "tools/e2e/package.json";
const E2E_NODE_LOCKFILE: &str = "tools/e2e/package-lock.json";
const E2E_NODE_MODULES: &str = "tools/e2e/node_modules";
const E2E_PROJECT_DIR: &str = "tools/e2e";
const E2E_MANIFEST_NAME: &str = "ui-feedback-manifest.json";
const DESKTOP_TAURI_PACKAGE: &str = "desktop_tauri";

/// `cargo e2e ...`
pub struct E2eCommand;

/// Supported `cargo e2e` subcommands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum E2eOptions {
    List,
    Doctor,
    Run(E2eRunOptions),
    Promote(E2ePromoteOptions),
    Inspect(E2eInspectOptions),
    Help,
}

/// Typed `cargo e2e run` options.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct E2eRunOptions {
    pub profile: String,
    pub scenario: Option<String>,
    pub slice: Option<String>,
    pub dry_run: bool,
    pub debug: bool,
    pub no_diff: bool,
    pub base_url: Option<String>,
    pub artifact_dir: Option<PathBuf>,
    pub manifest_out: Option<PathBuf>,
}

/// Typed `cargo e2e promote` options.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct E2ePromoteOptions {
    pub profile: String,
    pub scenario: Option<String>,
    pub slice: Option<String>,
    pub source_run: String,
}

/// Typed `cargo e2e inspect` options.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct E2eInspectOptions {
    pub run: String,
}

#[derive(Clone, Debug, Deserialize)]
struct E2eProfilesFile {
    profile: BTreeMap<String, E2eProfileSpec>,
}

#[derive(Clone, Debug, Deserialize)]
struct E2eScenariosFile {
    scenario: BTreeMap<String, E2eScenarioSpec>,
    scenario_set: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Deserialize)]
struct E2eProfileSpec {
    backend: E2eBackend,
    target: E2eTarget,
    scenario_set: String,
    browsers: Vec<String>,
    headless: bool,
    workers: WorkerCount,
    retries: u32,
    trace: String,
    slow_mo_ms: Option<u64>,
    supported_os: Vec<SupportedOs>,
    #[serde(default)]
    mode: Option<E2eRunMode>,
    #[serde(default)]
    default_viewport_set: Option<String>,
    #[serde(default)]
    artifact_level: Option<E2eArtifactLevel>,
    #[serde(default)]
    capture_accessibility: Option<bool>,
    #[serde(default)]
    capture_dom: Option<bool>,
    #[serde(default)]
    capture_layout: Option<bool>,
    #[serde(default)]
    capture_console: Option<bool>,
    #[serde(default)]
    capture_network: Option<bool>,
    #[serde(default)]
    snapshot_diff: Option<E2eDiffStrategy>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
struct E2eScenarioSpec {
    summary: String,
    backends: Vec<E2eBackend>,
    targets: Vec<E2eTarget>,
    tags: Option<Vec<String>>,
    #[serde(default)]
    slice_family: Option<String>,
    #[serde(default)]
    viewports: Vec<String>,
    #[serde(default)]
    diff_strategy: Option<E2eDiffStrategy>,
    #[serde(default)]
    baseline: Option<bool>,
    #[serde(default)]
    states: Vec<String>,
    #[serde(default)]
    entry: Option<String>,
    #[serde(default)]
    setup: Option<String>,
    #[serde(default)]
    assertions: Vec<String>,
    #[serde(default)]
    slices: Vec<String>,
}

#[derive(Clone, Debug)]
struct LoadedE2eConfig {
    profiles: E2eProfilesFile,
    scenarios: E2eScenariosFile,
}

#[derive(Clone, Debug)]
struct ResolvedE2eRun {
    profile_name: String,
    profile: E2eProfileSpec,
    scenario_ids: Vec<String>,
    slice: Option<String>,
    debug: bool,
    no_diff: bool,
    manifest_out: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum E2eBackend {
    Playwright,
    TauriWebdriver,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum E2eTarget {
    Browser,
    Tauri,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum SupportedOs {
    Macos,
    Linux,
    Windows,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
enum WorkerCount {
    Fixed(u32),
    Named(String),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum E2eRunMode {
    Validate,
    Debug,
    Capture,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum E2eArtifactLevel {
    Minimal,
    Standard,
    Full,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum E2eDiffStrategy {
    Pixel,
    Dom,
    Hybrid,
    None,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
struct UiFeedbackManifest {
    schema_version: u32,
    run_id: String,
    profile: String,
    mode: String,
    base_url: String,
    started_at: String,
    finished_at: Option<String>,
    status: String,
    artifact_root: String,
    #[serde(default)]
    environment: Option<UiFeedbackEnvironment>,
    summary: UiFeedbackSummary,
    scenarios: Vec<UiFeedbackScenarioResult>,
}

#[derive(Clone, Debug, Deserialize)]
struct UiFeedbackSummary {
    scenario_count: usize,
    slice_count: usize,
    passed: usize,
    failed: usize,
    diff_failures: usize,
    assertion_failures: usize,
    console_errors: usize,
    #[serde(default)]
    flaky_slice_count: usize,
    #[serde(default)]
    retry_success_count: usize,
    #[serde(default)]
    failure_categories: BTreeMap<String, usize>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
struct UiFeedbackScenarioResult {
    id: String,
    slice_id: String,
    browser: String,
    #[serde(default = "default_attempt")]
    attempt: u32,
    viewport: UiFeedbackViewport,
    status: String,
    baseline_enabled: bool,
    diff_strategy: String,
    artifacts: UiFeedbackArtifacts,
    assertions: Vec<UiFeedbackAssertionResult>,
    #[serde(default)]
    failure_categories: Vec<String>,
    failures: Vec<UiFeedbackFailure>,
}

#[derive(Clone, Debug, Deserialize)]
struct UiFeedbackViewport {
    id: String,
    width: u32,
    height: u32,
    device_scale_factor: u32,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
struct UiFeedbackArtifacts {
    screenshot: Option<String>,
    dom_snapshot: Option<String>,
    a11y_tree: Option<String>,
    layout_metrics: Option<String>,
    #[serde(default)]
    style_snapshot: Option<String>,
    console_log: Option<String>,
    page_errors: Option<String>,
    network_log: Option<String>,
    trace: Option<String>,
    pixel_diff: Option<String>,
    structured_diff: Option<String>,
    #[serde(default)]
    timing_snapshot: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
struct UiFeedbackAssertionResult {
    kind: String,
    target: String,
    status: String,
    detail: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize)]
struct UiFeedbackFailure {
    code: String,
    #[serde(default)]
    category: String,
    message: String,
    detail: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct UiFeedbackEnvironment {
    browser: String,
    color_scheme: String,
    reduced_motion: String,
    fixed_epoch: String,
    deterministic_math_random: bool,
    motion_frozen: bool,
    viewport_set: String,
    workers: u32,
}

#[derive(Clone, Debug, Serialize)]
struct BaselineManifest {
    schema_version: u32,
    promoted_at: String,
    source_run_id: String,
    profile: String,
    scenario_id: String,
    slice_id: String,
    browser: String,
    viewport: BaselineViewport,
    hashes: BaselineHashes,
}

#[derive(Clone, Debug, Serialize)]
struct BaselineViewport {
    id: String,
    width: u32,
    height: u32,
    device_scale_factor: u32,
}

#[derive(Clone, Debug, Serialize)]
struct BaselineHashes {
    screenshot_sha256: String,
    dom_sha256: String,
    a11y_sha256: String,
    layout_sha256: String,
    style_sha256: String,
}

fn default_attempt() -> u32 {
    1
}

impl XtaskCommand for E2eCommand {
    type Options = E2eOptions;

    fn parse(args: &[String]) -> XtaskResult<Self::Options> {
        parse_e2e_options(args)
    }

    fn run(ctx: &CommandContext, options: Self::Options) -> XtaskResult<()> {
        match options {
            E2eOptions::List => e2e_list(ctx),
            E2eOptions::Doctor => e2e_doctor(ctx),
            E2eOptions::Run(options) => e2e_run(ctx, options),
            E2eOptions::Promote(options) => e2e_promote(ctx, options),
            E2eOptions::Inspect(options) => e2e_inspect(ctx, options),
            E2eOptions::Help => {
                print_e2e_usage();
                Ok(())
            }
        }
    }
}

fn parse_e2e_options(args: &[String]) -> XtaskResult<E2eOptions> {
    match args.first().map(String::as_str) {
        None | Some("list") => Ok(E2eOptions::List),
        Some("doctor") => Ok(E2eOptions::Doctor),
        Some("run") => Ok(E2eOptions::Run(parse_run_options(&args[1..])?)),
        Some("promote") => Ok(E2eOptions::Promote(parse_promote_options(&args[1..])?)),
        Some("inspect") => Ok(E2eOptions::Inspect(parse_inspect_options(&args[1..])?)),
        Some("help" | "--help" | "-h") => Ok(E2eOptions::Help),
        Some(other) => Err(XtaskError::validation(format!(
            "unknown e2e subcommand: {other}"
        ))),
    }
}

fn parse_run_options(args: &[String]) -> XtaskResult<E2eRunOptions> {
    let mut profile = None;
    let mut scenario = None;
    let mut slice = None;
    let mut dry_run = false;
    let mut debug = false;
    let mut no_diff = false;
    let mut base_url = None;
    let mut artifact_dir = None;
    let mut manifest_out = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--profile" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(XtaskError::validation(
                        "missing value for `--profile` in `cargo e2e run`",
                    ));
                };
                profile = Some(value.clone());
                index += 2;
            }
            "--scenario" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(XtaskError::validation(
                        "missing value for `--scenario` in `cargo e2e run`",
                    ));
                };
                scenario = Some(value.clone());
                index += 2;
            }
            "--slice" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(XtaskError::validation(
                        "missing value for `--slice` in `cargo e2e run`",
                    ));
                };
                slice = Some(value.clone());
                index += 2;
            }
            "--dry-run" => {
                dry_run = true;
                index += 1;
            }
            "--debug" => {
                debug = true;
                index += 1;
            }
            "--no-diff" => {
                no_diff = true;
                index += 1;
            }
            "--base-url" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(XtaskError::validation(
                        "missing value for `--base-url` in `cargo e2e run`",
                    ));
                };
                base_url = Some(value.clone());
                index += 2;
            }
            "--artifact-dir" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(XtaskError::validation(
                        "missing value for `--artifact-dir` in `cargo e2e run`",
                    ));
                };
                artifact_dir = Some(PathBuf::from(value));
                index += 2;
            }
            "--manifest-out" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(XtaskError::validation(
                        "missing value for `--manifest-out` in `cargo e2e run`",
                    ));
                };
                manifest_out = Some(PathBuf::from(value));
                index += 2;
            }
            "--help" | "-h" => return Ok(E2eRunOptions::help_defaults()),
            other => {
                return Err(XtaskError::validation(format!(
                    "unknown `cargo e2e run` argument: {other}"
                )));
            }
        }
    }

    let Some(profile) = profile else {
        return Err(XtaskError::validation(
            "`cargo e2e run` requires `--profile <name>`",
        ));
    };

    Ok(E2eRunOptions {
        profile,
        scenario,
        slice,
        dry_run,
        debug,
        no_diff,
        base_url,
        artifact_dir,
        manifest_out,
    })
}

fn parse_promote_options(args: &[String]) -> XtaskResult<E2ePromoteOptions> {
    let mut profile = None;
    let mut scenario = None;
    let mut slice = None;
    let mut source_run = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--profile" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(XtaskError::validation(
                        "missing value for `--profile` in `cargo e2e promote`",
                    ));
                };
                profile = Some(value.clone());
                index += 2;
            }
            "--scenario" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(XtaskError::validation(
                        "missing value for `--scenario` in `cargo e2e promote`",
                    ));
                };
                scenario = Some(value.clone());
                index += 2;
            }
            "--slice" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(XtaskError::validation(
                        "missing value for `--slice` in `cargo e2e promote`",
                    ));
                };
                slice = Some(value.clone());
                index += 2;
            }
            "--source-run" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(XtaskError::validation(
                        "missing value for `--source-run` in `cargo e2e promote`",
                    ));
                };
                source_run = Some(value.clone());
                index += 2;
            }
            "--help" | "-h" => {
                return Ok(E2ePromoteOptions {
                    profile: String::new(),
                    scenario: None,
                    slice: None,
                    source_run: String::new(),
                })
            }
            other => {
                return Err(XtaskError::validation(format!(
                    "unknown `cargo e2e promote` argument: {other}"
                )));
            }
        }
    }

    let Some(profile) = profile else {
        return Err(XtaskError::validation(
            "`cargo e2e promote` requires `--profile <name>`",
        ));
    };
    let Some(source_run) = source_run else {
        return Err(XtaskError::validation(
            "`cargo e2e promote` requires `--source-run <path|run-id>`",
        ));
    };

    Ok(E2ePromoteOptions {
        profile,
        scenario,
        slice,
        source_run,
    })
}

fn parse_inspect_options(args: &[String]) -> XtaskResult<E2eInspectOptions> {
    let mut run = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--run" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(XtaskError::validation(
                        "missing value for `--run` in `cargo e2e inspect`",
                    ));
                };
                run = Some(value.clone());
                index += 2;
            }
            "--help" | "-h" => return Ok(E2eInspectOptions { run: String::new() }),
            other => {
                return Err(XtaskError::validation(format!(
                    "unknown `cargo e2e inspect` argument: {other}"
                )));
            }
        }
    }

    let Some(run) = run else {
        return Err(XtaskError::validation(
            "`cargo e2e inspect` requires `--run <path|run-id>`",
        ));
    };

    Ok(E2eInspectOptions { run })
}

impl E2eRunOptions {
    fn help_defaults() -> Self {
        Self {
            profile: String::new(),
            scenario: None,
            slice: None,
            dry_run: false,
            debug: false,
            no_diff: false,
            base_url: None,
            artifact_dir: None,
            manifest_out: None,
        }
    }
}

fn print_e2e_usage() {
    eprintln!(
        "Usage: cargo xtask e2e <subcommand>\n\
         \n\
         Subcommands:\n\
           list                  Show configured E2E profiles and scenario sets\n\
           doctor                Check local E2E prerequisites and config wiring\n\
           inspect --run <id>    Summarize a prior UI feedback manifest\n\
           promote --profile <name> [--scenario <id>] [--slice <id>] --source-run <path|run-id>\n\
                                 Promote approved UI feedback baselines from a completed run\n\
           run --profile <name> [--scenario <id>] [--slice <id>] [--base-url <url>] [--artifact-dir <path>] [--manifest-out <path>] [--debug] [--no-diff] [--dry-run]\n\
                                 Resolve a profile/scenario selection and execute the Playwright harness\n"
    );
}

fn e2e_list(ctx: &CommandContext) -> XtaskResult<()> {
    let config = load_e2e_config(ctx)?;
    validate_e2e_config(&config)?;

    println!("E2E profiles:");
    for (name, profile) in &config.profiles.profile {
        let browsers = if profile.browsers.is_empty() {
            "(n/a)".to_string()
        } else {
            profile.browsers.join(",")
        };
        println!(
            "  - {name}: backend={}, target={}, scenario_set={}, browsers={}, headless={}, workers={}, retries={}, trace={}, mode={}, viewport_set={}, artifact_level={}, snapshot_diff={}",
            profile.backend,
            profile.target,
            profile.scenario_set,
            browsers,
            profile.headless,
            profile.workers,
            profile.retries,
            profile.trace,
            profile.effective_mode(),
            profile.effective_viewport_set(),
            profile.effective_artifact_level(),
            profile.effective_snapshot_diff()
        );
    }

    println!("\nE2E scenario sets:");
    for (name, scenarios) in &config.scenarios.scenario_set {
        println!("  - {name}: {}", scenarios.join(", "));
    }

    println!("\nE2E scenarios:");
    for (name, scenario) in &config.scenarios.scenario {
        println!(
            "  - {name}: summary={}, backends={}, targets={}, tags={}, slices={}, viewports={}, baseline={}, diff={}",
            scenario.summary,
            join_display(&scenario.backends),
            join_display(&scenario.targets),
            join_strings(scenario.tags.as_deref().unwrap_or(&[])),
            join_strings(&scenario.slices),
            join_strings(&scenario.viewports),
            scenario.baseline.unwrap_or(false),
            scenario
                .diff_strategy
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "inherit".into())
        );
    }

    Ok(())
}

fn e2e_doctor(ctx: &CommandContext) -> XtaskResult<()> {
    let config = load_e2e_config(ctx)?;
    validate_e2e_config(&config)?;

    let cargo_available = ctx.process().command_available("cargo");
    let node_available = ctx.process().command_available("node");
    let npm_available = ctx.process().command_available("npm");
    let trunk_available = ctx.process().command_available("trunk");
    let package_json = ctx.artifacts().path(E2E_NODE_PACKAGE);
    let lockfile = ctx.artifacts().path(E2E_NODE_LOCKFILE);
    let node_modules = ctx.artifacts().path(E2E_NODE_MODULES);
    let baselines_dir = ctx.artifacts().e2e_baselines_dir();

    println!("E2E doctor:");
    println!("  - cargo: {}", status_word(cargo_available));
    println!("  - node: {}", status_word(node_available));
    println!("  - npm: {}", status_word(npm_available));
    println!("  - trunk: {}", status_word(trunk_available));
    println!(
        "  - tools/e2e package: {}",
        status_word(package_json.exists())
    );
    println!("  - tools/e2e lockfile: {}", status_word(lockfile.exists()));
    println!(
        "  - tools/e2e node_modules: {}",
        status_word(node_modules.exists())
    );
    println!(
        "  - tools/e2e baselines: {}",
        status_word(baselines_dir.exists())
    );

    let playwright_ready = if node_available && node_modules.exists() {
        playwright_launch_probe(ctx)
    } else {
        false
    };
    println!(
        "  - playwright browser launch probe: {}",
        status_word(playwright_ready)
    );

    let mut failures = Vec::new();
    if !cargo_available {
        failures.push("cargo");
    }
    if !node_available {
        failures.push("node");
    }
    if !npm_available {
        failures.push("npm");
    }
    if !trunk_available {
        failures.push("trunk");
    }
    if !package_json.exists() {
        failures.push("tools/e2e/package.json");
    }
    if !lockfile.exists() {
        failures.push("tools/e2e/package-lock.json");
    }
    if !baselines_dir.exists() {
        failures.push("tools/e2e/baselines");
    }

    if failures.is_empty() {
        if !node_modules.exists() {
            ctx.workflow().warn(
                "tools/e2e/node_modules is not present yet; `cargo e2e run` will bootstrap dependencies with `npm ci`",
            );
        } else if !playwright_ready {
            ctx.workflow().warn(
                "Playwright is installed but browser launch probing failed; rerun `npx playwright install --with-deps chromium` from tools/e2e if local browsers are missing",
            );
        }

        let current_os = SupportedOs::current();
        let desktop_profiles: Vec<_> = config
            .profiles
            .profile
            .iter()
            .filter_map(|(name, profile)| {
                if profile.backend == E2eBackend::TauriWebdriver {
                    Some(name.as_str())
                } else {
                    None
                }
            })
            .collect();
        let desktop_supported_here =
            matches!(current_os, SupportedOs::Linux | SupportedOs::Windows);
        let tauri_driver_available = ctx.process().command_available("tauri-driver");
        let native_webdriver_available = resolve_native_webdriver_binary(ctx).is_ok();
        if !desktop_profiles.is_empty() {
            if desktop_supported_here {
                if !tauri_driver_available || !native_webdriver_available {
                    ctx.workflow().warn(
                        "desktop Tauri/WebDriver profiles are configured, but local desktop-driver prerequisites are incomplete on this host",
                    );
                }
            } else {
                ctx.workflow().warn(&format!(
                    "desktop Tauri/WebDriver profiles ({}) are configured, but not supported on this host ({})",
                    desktop_profiles.join(", "),
                    current_os
                ));
            }
        }
        Ok(())
    } else {
        Err(XtaskError::environment(format!(
            "missing required E2E prerequisites: {}",
            failures.join(", ")
        ))
        .with_operation("cargo e2e doctor")
        .with_hint("run `cargo doctor`, install the missing tools, then rerun `cargo e2e doctor`"))
    }
}

fn e2e_run(ctx: &CommandContext, options: E2eRunOptions) -> XtaskResult<()> {
    if options.profile.is_empty() {
        print_e2e_usage();
        return Ok(());
    }

    let profile_name = options.profile.clone();
    ctx.workflow()
        .with_workflow_run("e2e", Some(profile_name.clone()), || {
            let mut resolved = None;
            let mut planned_run_dir = None;

            ctx.workflow()
                .run_timed_stage("Resolve E2E profile and scenario selection", || {
                    let config = load_e2e_config(ctx)?;
                    validate_e2e_config(&config)?;
                    let run = resolve_run(&config, &options)?;
                    resolved = Some(run);
                    Ok(())
                })?;

            let resolved = resolved.expect("resolved in previous stage");
            ctx.workflow().run_timed_stage("Report E2E execution plan", || {
                let run_dir = planned_e2e_run_dir(ctx, &resolved, &options);
                let node_lockfile = ctx.artifacts().path(E2E_NODE_LOCKFILE);
                let base_url = planned_base_url(ctx, &options)?;
                let manifest_out = resolved
                    .manifest_out
                    .as_ref()
                    .map(|path| ctx.artifacts().resolve_path(path).display().to_string())
                    .unwrap_or_else(|| "(none)".into());
                planned_run_dir = Some(run_dir.clone());

                println!("profile: {}", resolved.profile_name);
                println!("backend: {}", resolved.profile.backend);
                println!("target: {}", resolved.profile.target);
                println!("scenario set: {}", resolved.profile.scenario_set);
                println!("scenario ids: {}", resolved.scenario_ids.join(", "));
                println!(
                    "slice filter: {}",
                    resolved.slice.as_deref().unwrap_or("(all)")
                );
                println!(
                    "browsers: {}",
                    if resolved.profile.browsers.is_empty() {
                        "(n/a)".to_string()
                    } else {
                        resolved.profile.browsers.join(", ")
                    }
                );
                println!("headless: {}", resolved.effective_headless());
                println!("workers: {}", resolved.profile.workers);
                println!("trace: {}", resolved.profile.trace);
                println!("mode: {}", resolved.effective_mode());
                println!("viewport set: {}", resolved.profile.effective_viewport_set());
                println!("artifact level: {}", resolved.profile.effective_artifact_level());
                println!(
                    "snapshot diff: {}",
                    resolved.effective_snapshot_diff()
                );
                println!("base url: {base_url}");
                println!("artifact root: {}", run_dir.display());
                println!("manifest out: {manifest_out}");
                println!("node lockfile: {}", node_lockfile.display());

                if options.dry_run {
                    println!("mode: dry-run");
                    println!("status: configuration resolved successfully; no browser automation executed");
                    Ok(())
                } else {
                    run_e2e_backend(ctx, &resolved, &options, &run_dir)
                }
            })?;

            if !options.dry_run {
                let run_dir = planned_run_dir.expect("planned run dir set during execution plan");
                let manifest_path = resolve_manifest_path(ctx, &run_dir)?;
                summarize_manifest(ctx, &manifest_path)?;
                if let Some(path) = &resolved.manifest_out {
                    let out_path = ctx.artifacts().resolve_path(path);
                    if let Some(parent) = out_path.parent() {
                        ctx.artifacts().ensure_dir(parent)?;
                    }
                    fs::copy(&manifest_path, &out_path).map_err(|err| {
                        XtaskError::io(format!(
                            "failed to copy manifest to {}: {err}",
                            out_path.display()
                        ))
                    })?;
                    println!("copied manifest: {}", out_path.display());
                }
            }

            Ok(())
        })
}

pub(crate) fn run_named_profile(ctx: &CommandContext, profile: &str) -> XtaskResult<()> {
    e2e_run(
        ctx,
        E2eRunOptions {
            profile: profile.to_string(),
            scenario: None,
            slice: None,
            dry_run: false,
            debug: false,
            no_diff: false,
            base_url: None,
            artifact_dir: None,
            manifest_out: None,
        },
    )
}

fn e2e_inspect(ctx: &CommandContext, options: E2eInspectOptions) -> XtaskResult<()> {
    if options.run.is_empty() {
        print_e2e_usage();
        return Ok(());
    }
    let manifest_path = ctx.artifacts().resolve_manifest_reference(&options.run)?;
    summarize_manifest(ctx, &manifest_path)
}

fn e2e_promote(ctx: &CommandContext, options: E2ePromoteOptions) -> XtaskResult<()> {
    if options.profile.is_empty() {
        print_e2e_usage();
        return Ok(());
    }

    let manifest_path = ctx
        .artifacts()
        .resolve_manifest_reference(&options.source_run)?;
    let manifest = load_manifest(&manifest_path)?;
    if manifest.profile != options.profile {
        return Err(XtaskError::validation(format!(
            "run profile `{}` does not match requested promotion profile `{}`",
            manifest.profile, options.profile
        )));
    }
    if manifest.status != "passed" && manifest.status != "capture-complete" {
        return Err(XtaskError::validation(format!(
            "run `{}` is not promotable because status is `{}`",
            manifest.run_id, manifest.status
        )));
    }

    let candidates: Vec<_> = manifest
        .scenarios
        .iter()
        .filter(|entry| entry.baseline_enabled)
        .filter(|entry| {
            options
                .scenario
                .as_deref()
                .map(|scenario| scenario == entry.id)
                .unwrap_or(true)
        })
        .filter(|entry| {
            options
                .slice
                .as_deref()
                .map(|slice| slice == entry.slice_id)
                .unwrap_or(true)
        })
        .collect();

    if candidates.is_empty() {
        return Err(XtaskError::validation(
            "no baseline-enabled scenario results matched the requested promotion filter",
        ));
    }

    for entry in candidates {
        let baseline_dir = ctx.artifacts().e2e_baseline_target(
            &entry.id,
            &entry.slice_id,
            &entry.browser,
            &entry.viewport.id,
        );
        ctx.artifacts().ensure_dir(&baseline_dir)?;

        let screenshot = required_artifact_path(&entry.artifacts.screenshot, "screenshot")?;
        let dom = required_artifact_path(&entry.artifacts.dom_snapshot, "dom_snapshot")?;
        let a11y = required_artifact_path(&entry.artifacts.a11y_tree, "a11y_tree")?;
        let layout = required_artifact_path(&entry.artifacts.layout_metrics, "layout_metrics")?;
        let style = required_artifact_path(&entry.artifacts.style_snapshot, "style_snapshot")?;

        copy_baseline_artifact(&screenshot, &baseline_dir.join("screenshot.png"))?;
        copy_baseline_artifact(&dom, &baseline_dir.join("dom.json"))?;
        copy_baseline_artifact(&a11y, &baseline_dir.join("a11y.json"))?;
        copy_baseline_artifact(&layout, &baseline_dir.join("layout.json"))?;
        copy_baseline_artifact(&style, &baseline_dir.join("style.json"))?;

        let baseline_manifest = BaselineManifest {
            schema_version: 2,
            promoted_at: Utc::now().to_rfc3339(),
            source_run_id: manifest.run_id.clone(),
            profile: manifest.profile.clone(),
            scenario_id: entry.id.clone(),
            slice_id: entry.slice_id.clone(),
            browser: entry.browser.clone(),
            viewport: BaselineViewport {
                id: entry.viewport.id.clone(),
                width: entry.viewport.width,
                height: entry.viewport.height,
                device_scale_factor: entry.viewport.device_scale_factor,
            },
            hashes: BaselineHashes {
                screenshot_sha256: sha256_file(&baseline_dir.join("screenshot.png"))?,
                dom_sha256: sha256_file(&baseline_dir.join("dom.json"))?,
                a11y_sha256: sha256_file(&baseline_dir.join("a11y.json"))?,
                layout_sha256: sha256_file(&baseline_dir.join("layout.json"))?,
                style_sha256: sha256_file(&baseline_dir.join("style.json"))?,
            },
        };
        let manifest_json = serde_json::to_string_pretty(&baseline_manifest).map_err(|err| {
            XtaskError::io(format!("failed to serialize baseline manifest: {err}"))
        })?;
        fs::write(
            baseline_dir.join("manifest.json"),
            format!("{manifest_json}\n"),
        )
        .map_err(|err| {
            XtaskError::io(format!(
                "failed to write {}: {err}",
                baseline_dir.join("manifest.json").display()
            ))
        })?;
        println!(
            "promoted baseline: scenario={}, slice={}, browser={}, viewport={}, dir={}",
            entry.id,
            entry.slice_id,
            entry.browser,
            entry.viewport.id,
            baseline_dir.display()
        );
    }

    Ok(())
}

fn run_e2e_backend(
    ctx: &CommandContext,
    resolved: &ResolvedE2eRun,
    options: &E2eRunOptions,
    artifact_dir: &Path,
) -> XtaskResult<()> {
    match resolved.profile.backend {
        E2eBackend::Playwright => run_playwright_backend(ctx, resolved, options, artifact_dir),
        E2eBackend::TauriWebdriver => {
            run_tauri_webdriver_backend(ctx, resolved, options, artifact_dir)
        }
    }
}

fn run_tauri_webdriver_backend(
    ctx: &CommandContext,
    resolved: &ResolvedE2eRun,
    _options: &E2eRunOptions,
    artifact_dir: &Path,
) -> XtaskResult<()> {
    match SupportedOs::current() {
        SupportedOs::Macos => Err(XtaskError::unsupported_platform(format!(
            "E2E profile `{}` uses `tauri-webdriver`, which is not supported on macOS in this workflow; use a browser profile such as `local-dev`, `ci-headless`, or `cross-browser` instead",
            resolved.profile_name
        ))),
        SupportedOs::Linux | SupportedOs::Windows => {
            ensure_e2e_project_files(ctx)?;
            ctx.artifacts().ensure_dir(artifact_dir)?;
            ctx.workflow()
                .run_timed_stage("Bootstrap E2E Node dependencies", || ensure_node_modules(ctx))?;
            let dev_config = load_dev_server_config(ctx)?;
            let desktop_binary = build_desktop_tauri_binary(ctx)?;
            let native_driver = resolve_native_webdriver_binary(ctx)?;
            let mut frontend_server = None;
            ctx.workflow()
                .run_timed_stage("Start desktop E2E frontend server", || {
                    frontend_server =
                        Some(start_isolated_e2e_server(ctx, artifact_dir, &dev_config)?);
                    Ok(())
                })?;
            let frontend_server =
                frontend_server.expect("desktop frontend server set in previous stage");
            let mut tauri_driver = None;
            let mut tauri_driver_url = None;
            ctx.workflow()
                .run_timed_stage("Start tauri-driver", || {
                    let handle =
                        start_tauri_driver_server(ctx, artifact_dir, native_driver.as_path())?;
                    tauri_driver_url = Some(handle.base_url());
                    tauri_driver = Some(handle);
                    Ok(())
                })?;
            let tauri_driver_url =
                tauri_driver_url.expect("tauri driver url set in previous stage");
            let mut tauri_driver = tauri_driver.expect("tauri driver set in previous stage");
            let run_result = execute_tauri_webdriver_harness(
                ctx,
                resolved,
                artifact_dir,
                &tauri_driver_url,
                desktop_binary.as_path(),
                &frontend_server.base_url(),
            );

            match stop_background_http_server(&mut tauri_driver) {
                Ok(()) => {}
                Err(err) => {
                    if run_result.is_ok() {
                        return Err(err);
                    }
                    ctx.workflow().warn(&format!(
                        "tauri-driver cleanup failed after desktop E2E failure: {err}"
                    ));
                }
            }

            let mut frontend_server = frontend_server;
            match stop_isolated_e2e_server(ctx, &mut frontend_server) {
                Ok(()) => {}
                Err(err) => {
                    if run_result.is_ok() {
                        return Err(err);
                    }
                    ctx.workflow().warn(&format!(
                        "desktop frontend server cleanup failed after desktop E2E failure: {err}"
                    ));
                }
            }

            run_result
        }
    }
}

fn run_playwright_backend(
    ctx: &CommandContext,
    resolved: &ResolvedE2eRun,
    options: &E2eRunOptions,
    artifact_dir: &Path,
) -> XtaskResult<()> {
    ensure_e2e_project_files(ctx)?;
    ctx.artifacts().ensure_dir(artifact_dir)?;

    ctx.workflow()
        .run_timed_stage("Bootstrap E2E Node dependencies", || {
            ensure_node_modules(ctx)
        })?;

    if let Some(base_url) = &options.base_url {
        return execute_playwright_harness(ctx, resolved, artifact_dir, base_url);
    }

    let config = load_dev_server_config(ctx)?;
    let mut server_state = None;
    ctx.workflow()
        .run_timed_stage("Start isolated E2E dev server", || {
            server_state = Some(start_isolated_e2e_server(ctx, artifact_dir, &config)?);
            Ok(())
        })?;
    let mut server_state = server_state.expect("server state captured in previous stage");
    let run_result =
        execute_playwright_harness(ctx, resolved, artifact_dir, &server_state.base_url());

    match stop_isolated_e2e_server(ctx, &mut server_state) {
        Ok(()) => {}
        Err(err) => {
            if run_result.is_ok() {
                return Err(err);
            }
            ctx.workflow().warn(&format!(
                "isolated E2E server cleanup failed after E2E failure: {err}"
            ));
        }
    }

    run_result
}

fn load_e2e_config(ctx: &CommandContext) -> XtaskResult<LoadedE2eConfig> {
    let profiles = ConfigLoader::<E2eProfilesFile>::new(ctx.root(), E2E_PROFILES_CONFIG).load()?;
    let scenarios =
        ConfigLoader::<E2eScenariosFile>::new(ctx.root(), E2E_SCENARIOS_CONFIG).load()?;
    Ok(LoadedE2eConfig {
        profiles,
        scenarios,
    })
}

fn validate_e2e_config(config: &LoadedE2eConfig) -> XtaskResult<()> {
    let allowed_viewports = ["desktop", "tablet", "mobile"];

    for (profile_name, profile) in &config.profiles.profile {
        let Some(scenarios) = config.scenarios.scenario_set.get(&profile.scenario_set) else {
            return Err(XtaskError::config(format!(
                "E2E profile `{profile_name}` references unknown scenario set `{}`",
                profile.scenario_set
            )));
        };

        if profile.backend == E2eBackend::Playwright && profile.browsers.is_empty() {
            return Err(XtaskError::config(format!(
                "E2E profile `{profile_name}` must define at least one browser for the Playwright backend"
            )));
        }

        if profile.supported_os.is_empty() {
            return Err(XtaskError::config(format!(
                "E2E profile `{profile_name}` must define at least one supported OS"
            )));
        }

        if !matches!(profile.trace.as_str(), "on" | "retain-on-failure" | "off") {
            return Err(XtaskError::config(format!(
                "E2E profile `{profile_name}` has invalid trace policy `{}` (expected `on`, `retain-on-failure`, or `off`)",
                profile.trace
            )));
        }

        if !matches!(
            profile.effective_viewport_set(),
            "desktop-standard" | "responsive-core" | "debug-focused"
        ) {
            return Err(XtaskError::config(format!(
                "E2E profile `{profile_name}` has invalid default viewport set `{}`",
                profile.effective_viewport_set()
            )));
        }

        for scenario_id in scenarios {
            let Some(scenario) = config.scenarios.scenario.get(scenario_id) else {
                return Err(XtaskError::config(format!(
                    "E2E scenario set `{}` references unknown scenario `{scenario_id}`",
                    profile.scenario_set
                )));
            };

            if !scenario.backends.contains(&profile.backend) {
                return Err(XtaskError::config(format!(
                    "E2E scenario `{scenario_id}` does not support backend `{}` required by profile `{profile_name}`",
                    profile.backend
                )));
            }

            if !scenario.targets.contains(&profile.target) {
                return Err(XtaskError::config(format!(
                    "E2E scenario `{scenario_id}` does not support target `{}` required by profile `{profile_name}`",
                    profile.target
                )));
            }

            let mut seen_slices = BTreeSet::new();
            for slice in &scenario.slices {
                if !seen_slices.insert(slice) {
                    return Err(XtaskError::config(format!(
                        "E2E scenario `{scenario_id}` defines duplicate slice `{slice}`"
                    )));
                }
            }
            for viewport in &scenario.viewports {
                if !allowed_viewports.contains(&viewport.as_str()) {
                    return Err(XtaskError::config(format!(
                        "E2E scenario `{scenario_id}` defines unsupported viewport `{viewport}`"
                    )));
                }
            }
        }
    }

    Ok(())
}

fn resolve_run(config: &LoadedE2eConfig, options: &E2eRunOptions) -> XtaskResult<ResolvedE2eRun> {
    let Some(profile) = config.profiles.profile.get(&options.profile) else {
        return Err(XtaskError::validation(format!(
            "unknown E2E profile `{}`",
            options.profile
        )));
    };

    ensure_supported_os(profile, &options.profile)?;

    let scenario_ids = if let Some(scenario_id) = &options.scenario {
        let Some(scenario) = config.scenarios.scenario.get(scenario_id) else {
            return Err(XtaskError::validation(format!(
                "unknown E2E scenario `{scenario_id}`"
            )));
        };
        if !scenario.backends.contains(&profile.backend)
            || !scenario.targets.contains(&profile.target)
        {
            return Err(XtaskError::validation(format!(
                "scenario `{scenario_id}` is incompatible with profile `{}`",
                options.profile
            )));
        }
        if let Some(slice) = &options.slice {
            if !scenario.slices.is_empty()
                && !scenario.slices.iter().any(|candidate| candidate == slice)
            {
                return Err(XtaskError::validation(format!(
                    "scenario `{scenario_id}` does not define slice `{slice}`"
                )));
            }
        }
        vec![scenario_id.clone()]
    } else {
        config
            .scenarios
            .scenario_set
            .get(&profile.scenario_set)
            .cloned()
            .ok_or_else(|| {
                XtaskError::config(format!(
                    "E2E profile `{}` references unknown scenario set `{}`",
                    options.profile, profile.scenario_set
                ))
            })?
    };

    Ok(ResolvedE2eRun {
        profile_name: options.profile.clone(),
        profile: profile.clone(),
        scenario_ids,
        slice: options.slice.clone(),
        debug: options.debug,
        no_diff: options.no_diff,
        manifest_out: options.manifest_out.clone(),
    })
}

fn ensure_supported_os(profile: &E2eProfileSpec, profile_name: &str) -> XtaskResult<()> {
    let current = SupportedOs::current();
    if profile.supported_os.contains(&current) {
        Ok(())
    } else if profile.backend == E2eBackend::TauriWebdriver && current == SupportedOs::Macos {
        Err(XtaskError::unsupported_platform(format!(
            "E2E profile `{profile_name}` uses `tauri-webdriver`, which is not supported on macOS in this workflow; use a browser profile such as `local-dev`, `ci-headless`, or `cross-browser` instead"
        )))
    } else {
        Err(XtaskError::unsupported_platform(format!(
            "E2E profile `{profile_name}` is not supported on {}",
            current
        )))
    }
}

fn status_word(available: bool) -> &'static str {
    if available {
        "ok"
    } else {
        "missing"
    }
}

fn planned_e2e_run_dir(
    ctx: &CommandContext,
    resolved: &ResolvedE2eRun,
    options: &E2eRunOptions,
) -> PathBuf {
    if let Some(path) = &options.artifact_dir {
        ctx.artifacts().resolve_path(path)
    } else {
        ctx.artifacts().e2e_runs_dir().join(format!(
            "{}-{}",
            unix_timestamp_millis(),
            resolved.profile_name
        ))
    }
}

fn ensure_e2e_project_files(ctx: &CommandContext) -> XtaskResult<()> {
    let package_json = ctx.artifacts().path(E2E_NODE_PACKAGE);
    let lockfile = ctx.artifacts().path(E2E_NODE_LOCKFILE);

    if !package_json.exists() {
        return Err(
            XtaskError::environment(format!("missing {}", package_json.display()))
                .with_operation("cargo e2e run")
                .with_hint(
                    "restore the versioned Node E2E project before running browser automation",
                ),
        );
    }

    if !lockfile.exists() {
        return Err(
            XtaskError::environment(format!("missing {}", lockfile.display()))
                .with_operation("cargo e2e run")
                .with_hint(
                    "generate and commit the Playwright lockfile before running browser automation",
                ),
        );
    }

    Ok(())
}

fn ensure_node_modules(ctx: &CommandContext) -> XtaskResult<()> {
    let node_modules = ctx.artifacts().path(E2E_NODE_MODULES);
    if node_modules.exists() {
        return Ok(());
    }

    ctx.process().run(
        &ctx.root().join(E2E_PROJECT_DIR),
        "npm",
        vec!["ci", "--no-fund", "--no-audit"],
    )
}

fn build_desktop_tauri_binary(ctx: &CommandContext) -> XtaskResult<PathBuf> {
    ctx.workflow()
        .run_timed_stage("Build desktop Tauri binary", || {
            ctx.process().run(
                ctx.root(),
                "cargo",
                vec!["build", "-p", DESKTOP_TAURI_PACKAGE],
            )
        })?;
    let binary_name = if cfg!(windows) {
        "desktop_tauri.exe"
    } else {
        "desktop_tauri"
    };
    let binary = ctx.root().join("target").join("debug").join(binary_name);
    if binary.exists() {
        Ok(binary)
    } else {
        Err(XtaskError::process_exit(format!(
            "desktop Tauri binary was not produced at {}",
            binary.display()
        )))
    }
}

fn resolve_native_webdriver_binary(ctx: &CommandContext) -> XtaskResult<PathBuf> {
    let candidates = match SupportedOs::current() {
        SupportedOs::Windows => vec!["msedgedriver", "chromedriver"],
        SupportedOs::Linux => vec!["geckodriver", "chromedriver"],
        SupportedOs::Macos => Vec::new(),
    };
    for candidate in candidates {
        let lookup_program = if cfg!(windows) { "where" } else { "which" };
        let path = ctx
            .process()
            .capture_stdout_line(lookup_program, &[candidate]);
        if path != "unavailable" {
            return Ok(PathBuf::from(path));
        }
    }
    Err(XtaskError::environment(
        "no supported native WebDriver bridge was found on PATH for desktop E2E",
    )
    .with_operation("cargo e2e run")
    .with_hint("install `geckodriver`, `chromedriver`, or `msedgedriver` as appropriate for the host platform"))
}

fn planned_base_url(ctx: &CommandContext, options: &E2eRunOptions) -> XtaskResult<String> {
    if let Some(base_url) = &options.base_url {
        return Ok(base_url.clone());
    }

    let config = load_dev_server_config(ctx)?;
    let host = normalize_host_for_url(&config.default_host);
    Ok(format!("http://{host}:<ephemeral>/"))
}

fn start_isolated_e2e_server(
    ctx: &CommandContext,
    artifact_dir: &Path,
    config: &DevServerConfig,
) -> XtaskResult<BackgroundHttpServerHandle> {
    let host = config.default_host.clone();
    let public_host = normalize_host_for_url(&host);
    let port = allocate_local_http_port(&host)?;
    let server_dir = artifact_dir.join("server");
    ctx.artifacts().ensure_dir(&server_dir)?;
    let log_path = server_dir.join("trunk.log");
    let spec = BackgroundHttpServerSpec {
        program: "trunk".into(),
        args: vec![
            "serve".to_string(),
            "index.html".to_string(),
            "--no-sri=true".to_string(),
            "--ignore".to_string(),
            "dist".to_string(),
            "--address".to_string(),
            host,
            "--port".to_string(),
            port.to_string(),
        ],
        cwd: ctx.root().join("crates/site"),
        bind_host: config.default_host.clone(),
        public_host,
        port,
        log_path,
        startup_timeout: config.start_poll(),
        shutdown_timeout: config.stop_timeout(),
    };
    start_background_http_server(ctx.process(), &spec)
}

fn start_tauri_driver_server(
    ctx: &CommandContext,
    artifact_dir: &Path,
    native_driver: &Path,
) -> XtaskResult<BackgroundHttpServerHandle> {
    let server_dir = artifact_dir.join("driver");
    ctx.artifacts().ensure_dir(&server_dir)?;
    let log_path = server_dir.join("tauri-driver.log");
    let port = allocate_local_http_port("127.0.0.1")?;
    let spec = BackgroundHttpServerSpec {
        program: "tauri-driver".into(),
        args: vec![
            "--port".to_string(),
            port.to_string(),
            "--native-driver".to_string(),
            native_driver.display().to_string(),
        ],
        cwd: ctx.root().to_path_buf(),
        bind_host: "127.0.0.1".into(),
        public_host: "127.0.0.1".into(),
        port,
        log_path,
        startup_timeout: Duration::from_secs(10),
        shutdown_timeout: Duration::from_secs(5),
    };
    start_background_http_server(ctx.process(), &spec)
}

fn stop_isolated_e2e_server(
    ctx: &CommandContext,
    state: &mut BackgroundHttpServerHandle,
) -> XtaskResult<()> {
    ctx.workflow()
        .run_timed_stage("Stop isolated E2E dev server", || {
            stop_background_http_server(state)
        })
}

fn execute_playwright_harness(
    ctx: &CommandContext,
    resolved: &ResolvedE2eRun,
    artifact_dir: &Path,
    base_url: &str,
) -> XtaskResult<()> {
    let baseline_root = ctx.artifacts().e2e_baselines_dir();
    ctx.workflow()
        .run_timed_stage("Execute Playwright harness", || {
            let scenario_ids = resolved.scenario_ids.join(",");
            let browsers = resolved.profile.browsers.join(",");
            let headless = if resolved.effective_headless() {
                "true"
            } else {
                "false"
            };
            let retries = resolved.profile.retries.to_string();
            let slow_mo_ms = resolved.effective_slow_mo_ms().to_string();
            let artifact_dir_string = artifact_dir.display().to_string();
            let slice = resolved.slice.as_deref().unwrap_or("");
            let no_diff = if resolved.no_diff { "true" } else { "false" };
            let capture_network = if resolved.profile.capture_network.unwrap_or(false) {
                "true"
            } else {
                "false"
            };

            ctx.process().run_owned_with_env(
                &ctx.root().join(E2E_PROJECT_DIR),
                "node",
                vec!["src/run.mjs".into()],
                &[
                    ("OS_E2E_PROFILE", resolved.profile_name.as_str()),
                    ("OS_E2E_BACKEND", "playwright"),
                    ("OS_E2E_TARGET", "browser"),
                    ("OS_E2E_BASE_URL", base_url),
                    ("OS_E2E_ARTIFACT_DIR", artifact_dir_string.as_str()),
                    (
                        "OS_E2E_BASELINE_ROOT",
                        baseline_root.display().to_string().as_str(),
                    ),
                    (
                        "OS_E2E_MANIFEST_PATH",
                        &artifact_dir
                            .join("reports")
                            .join(E2E_MANIFEST_NAME)
                            .display()
                            .to_string(),
                    ),
                    ("OS_E2E_SCENARIO_IDS", scenario_ids.as_str()),
                    ("OS_E2E_SLICE_ID", slice),
                    ("OS_E2E_BROWSERS", browsers.as_str()),
                    ("OS_E2E_HEADLESS", headless),
                    ("OS_E2E_TRACE", resolved.profile.trace.as_str()),
                    ("OS_E2E_RETRIES", retries.as_str()),
                    ("OS_E2E_SLOW_MO_MS", slow_mo_ms.as_str()),
                    (
                        "OS_E2E_MODE",
                        resolved.effective_mode().to_string().as_str(),
                    ),
                    (
                        "OS_E2E_VIEWPORT_SET",
                        resolved.profile.effective_viewport_set(),
                    ),
                    (
                        "OS_E2E_ARTIFACT_LEVEL",
                        resolved
                            .profile
                            .effective_artifact_level()
                            .to_string()
                            .as_str(),
                    ),
                    (
                        "OS_E2E_CAPTURE_ACCESSIBILITY",
                        bool_env(resolved.profile.capture_accessibility.unwrap_or(true)),
                    ),
                    (
                        "OS_E2E_CAPTURE_DOM",
                        bool_env(resolved.profile.capture_dom.unwrap_or(true)),
                    ),
                    (
                        "OS_E2E_CAPTURE_LAYOUT",
                        bool_env(resolved.profile.capture_layout.unwrap_or(true)),
                    ),
                    (
                        "OS_E2E_CAPTURE_CONSOLE",
                        bool_env(resolved.profile.capture_console.unwrap_or(true)),
                    ),
                    (
                        "OS_E2E_CAPTURE_NETWORK",
                        bool_env(capture_network == "true"),
                    ),
                    (
                        "OS_E2E_SNAPSHOT_DIFF",
                        resolved.effective_snapshot_diff().to_string().as_str(),
                    ),
                    ("OS_E2E_NO_DIFF", no_diff),
                ],
            )
        })
}

fn execute_tauri_webdriver_harness(
    ctx: &CommandContext,
    resolved: &ResolvedE2eRun,
    artifact_dir: &Path,
    driver_url: &str,
    desktop_binary: &Path,
    frontend_base_url: &str,
) -> XtaskResult<()> {
    ctx.workflow()
        .run_timed_stage("Execute tauri-webdriver harness", || {
            let scenario_ids = resolved.scenario_ids.join(",");
            let retries = resolved.profile.retries.to_string();
            let artifact_dir_string = artifact_dir.display().to_string();
            let desktop_binary_string = desktop_binary.display().to_string();

            ctx.process().run_owned_with_env(
                &ctx.root().join(E2E_PROJECT_DIR),
                "node",
                vec!["src/run-desktop.mjs".into()],
                &[
                    ("OS_E2E_PROFILE", resolved.profile_name.as_str()),
                    ("OS_E2E_BACKEND", "tauri-webdriver"),
                    ("OS_E2E_TARGET", "tauri"),
                    ("OS_E2E_ARTIFACT_DIR", artifact_dir_string.as_str()),
                    ("OS_E2E_SCENARIO_IDS", scenario_ids.as_str()),
                    ("OS_E2E_RETRIES", retries.as_str()),
                    ("OS_E2E_TAURI_DRIVER_URL", driver_url),
                    ("OS_E2E_DESKTOP_BINARY", desktop_binary_string.as_str()),
                    ("OS_E2E_BASE_URL", frontend_base_url),
                ],
            )
        })
}

fn summarize_manifest(_ctx: &CommandContext, manifest_path: &Path) -> XtaskResult<()> {
    let manifest = load_manifest(manifest_path)?;
    println!("manifest: {}", manifest_path.display());
    println!("run id: {}", manifest.run_id);
    println!("schema: {}", manifest.schema_version);
    println!("profile: {}", manifest.profile);
    println!("mode: {}", manifest.mode);
    println!("status: {}", manifest.status);
    println!("artifact root: {}", manifest.artifact_root);
    if let Some(environment) = &manifest.environment {
        println!(
            "environment: browser={}, color_scheme={}, reduced_motion={}, epoch={}, deterministic_random={}, motion_frozen={}, viewport_set={}, workers={}",
            environment.browser,
            environment.color_scheme,
            environment.reduced_motion,
            environment.fixed_epoch,
            environment.deterministic_math_random,
            environment.motion_frozen,
            environment.viewport_set,
            environment.workers
        );
    }
    println!(
        "summary: scenarios={}, slices={}, passed={}, failed={}, diff_failures={}, assertion_failures={}, console_errors={}, flaky_slices={}, retry_successes={}",
        manifest.summary.scenario_count,
        manifest.summary.slice_count,
        manifest.summary.passed,
        manifest.summary.failed,
        manifest.summary.diff_failures,
        manifest.summary.assertion_failures,
        manifest.summary.console_errors,
        manifest.summary.flaky_slice_count,
        manifest.summary.retry_success_count
    );
    if !manifest.summary.failure_categories.is_empty() {
        println!("failure categories:");
        for (category, count) in &manifest.summary.failure_categories {
            println!("  - {category}: {count}");
        }
    }
    let mut grouped = BTreeMap::<String, Vec<&UiFeedbackScenarioResult>>::new();
    for failure in manifest
        .scenarios
        .iter()
        .filter(|entry| entry.status != "passed")
    {
        if failure.failure_categories.is_empty() {
            grouped
                .entry("uncategorized".into())
                .or_default()
                .push(failure);
            continue;
        }
        for category in &failure.failure_categories {
            grouped.entry(category.clone()).or_default().push(failure);
        }
    }
    for (category, failures) in grouped {
        println!("category: {category}");
        for failure in failures {
            println!(
                "  failure: scenario={}, slice={}, browser={}, viewport={}, attempt={}, diff={}, failures={}",
                failure.id,
                failure.slice_id,
                failure.browser,
                failure.viewport.id,
                failure.attempt,
                failure.diff_strategy,
                failure
                    .failures
                    .iter()
                    .map(|item| format!("{}:{}:{}", item.category, item.code, item.message))
                    .collect::<Vec<_>>()
                    .join(" | ")
            );
            if let Some(path) = preferred_failure_artifact(failure, &category) {
                println!("    artifact: {path}");
            }
            if let Some(path) = &failure.artifacts.trace {
                println!("    trace: {path}");
            }
        }
    }
    if manifest.summary.diff_failures > 0
        && manifest.summary.failed == manifest.summary.diff_failures
    {
        println!("hint: if the visual/structural changes are intentional, review the artifacts and run `cargo e2e promote --profile {} --source-run {}`", manifest.profile, manifest.run_id);
    }
    Ok(())
}

fn preferred_failure_artifact<'a>(
    failure: &'a UiFeedbackScenarioResult,
    category: &str,
) -> Option<&'a str> {
    match category {
        "visual-regression" => failure
            .artifacts
            .pixel_diff
            .as_deref()
            .or(failure.artifacts.structured_diff.as_deref()),
        "ui-contract-violation" => failure
            .artifacts
            .style_snapshot
            .as_deref()
            .or(failure.artifacts.structured_diff.as_deref())
            .or(failure.artifacts.dom_snapshot.as_deref()),
        "javascript-runtime" => failure.artifacts.page_errors.as_deref(),
        "network-failure" => failure.artifacts.network_log.as_deref(),
        "readiness-timeout" | "race-condition" => failure.artifacts.timing_snapshot.as_deref(),
        "baseline-missing" => failure
            .artifacts
            .pixel_diff
            .as_deref()
            .or(failure.artifacts.structured_diff.as_deref()),
        _ => failure
            .artifacts
            .structured_diff
            .as_deref()
            .or(failure.artifacts.pixel_diff.as_deref())
            .or(failure.artifacts.timing_snapshot.as_deref()),
    }
}

fn resolve_manifest_path(_ctx: &CommandContext, artifact_dir: &Path) -> XtaskResult<PathBuf> {
    let manifest_path = artifact_dir.join("reports").join(E2E_MANIFEST_NAME);
    if manifest_path.exists() {
        Ok(manifest_path)
    } else {
        Err(XtaskError::io(format!(
            "expected manifest at {}",
            manifest_path.display()
        )))
    }
}

fn load_manifest(path: &Path) -> XtaskResult<UiFeedbackManifest> {
    let content = fs::read_to_string(path)
        .map_err(|err| XtaskError::io(format!("failed to read {}: {err}", path.display())))?;
    serde_json::from_str::<UiFeedbackManifest>(&content).map_err(|err| {
        XtaskError::io(format!(
            "failed to parse UI feedback manifest {}: {err}",
            path.display()
        ))
    })
}

fn copy_baseline_artifact(source: &Path, destination: &Path) -> XtaskResult<()> {
    fs::copy(source, destination).map_err(|err| {
        XtaskError::io(format!(
            "failed to copy {} to {}: {err}",
            source.display(),
            destination.display()
        ))
    })?;
    Ok(())
}

fn required_artifact_path(value: &Option<String>, name: &str) -> XtaskResult<PathBuf> {
    let Some(path) = value else {
        return Err(XtaskError::validation(format!(
            "manifest entry is missing required artifact `{name}`"
        )));
    };
    Ok(PathBuf::from(path))
}

fn sha256_file(path: &Path) -> XtaskResult<String> {
    let output = Command::new("shasum")
        .arg("-a")
        .arg("256")
        .arg(path)
        .output()
        .map_err(|err| XtaskError::process_launch(format!("failed to start `shasum`: {err}")))?;
    if !output.status.success() {
        return Err(XtaskError::process_exit(format!(
            "`shasum` exited with status {} while hashing {}",
            output.status,
            path.display()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string())
}

fn bool_env(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

fn join_display<T: Display>(items: &[T]) -> String {
    items
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

fn join_strings(items: &[String]) -> String {
    if items.is_empty() {
        "(none)".into()
    } else {
        items.join(",")
    }
}

fn playwright_launch_probe(ctx: &CommandContext) -> bool {
    let output = Command::new("node")
        .current_dir(ctx.root().join(E2E_PROJECT_DIR))
        .arg("--input-type=module")
        .arg("-e")
        .arg("import { chromium } from 'playwright'; const browser = await chromium.launch({ headless: true }); await browser.close();")
        .output();
    output
        .map(|status| status.status.success())
        .unwrap_or(false)
}

impl E2eProfileSpec {
    fn effective_mode(&self) -> E2eRunMode {
        self.mode.clone().unwrap_or({
            if self.headless {
                E2eRunMode::Validate
            } else {
                E2eRunMode::Debug
            }
        })
    }

    fn effective_viewport_set(&self) -> &str {
        self.default_viewport_set
            .as_deref()
            .unwrap_or("responsive-core")
    }

    fn effective_artifact_level(&self) -> E2eArtifactLevel {
        self.artifact_level
            .clone()
            .unwrap_or(E2eArtifactLevel::Standard)
    }

    fn effective_snapshot_diff(&self) -> E2eDiffStrategy {
        self.snapshot_diff
            .clone()
            .unwrap_or(E2eDiffStrategy::Hybrid)
    }
}

impl ResolvedE2eRun {
    fn effective_headless(&self) -> bool {
        if self.debug {
            false
        } else {
            self.profile.headless
        }
    }

    fn effective_slow_mo_ms(&self) -> u64 {
        if self.debug {
            self.profile.slow_mo_ms.unwrap_or(250)
        } else {
            self.profile.slow_mo_ms.unwrap_or(0)
        }
    }

    fn effective_mode(&self) -> E2eRunMode {
        if self.debug {
            E2eRunMode::Debug
        } else if self.no_diff {
            E2eRunMode::Capture
        } else {
            self.profile.effective_mode()
        }
    }

    fn effective_snapshot_diff(&self) -> E2eDiffStrategy {
        if self.no_diff {
            E2eDiffStrategy::None
        } else {
            self.profile.effective_snapshot_diff()
        }
    }
}

impl SupportedOs {
    fn current() -> Self {
        match std::env::consts::OS {
            "macos" => Self::Macos,
            "windows" => Self::Windows,
            _ => Self::Linux,
        }
    }
}

impl Display for E2eBackend {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Playwright => write!(f, "playwright"),
            Self::TauriWebdriver => write!(f, "tauri-webdriver"),
        }
    }
}

impl Display for E2eTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Browser => write!(f, "browser"),
            Self::Tauri => write!(f, "tauri"),
        }
    }
}

impl Display for WorkerCount {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixed(count) => write!(f, "{count}"),
            Self::Named(name) => write!(f, "{name}"),
        }
    }
}

impl Display for SupportedOs {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Macos => write!(f, "macOS"),
            Self::Linux => write!(f, "Linux"),
            Self::Windows => write!(f, "Windows"),
        }
    }
}

impl Display for E2eRunMode {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validate => write!(f, "validate"),
            Self::Debug => write!(f, "debug"),
            Self::Capture => write!(f, "capture"),
        }
    }
}

impl Display for E2eArtifactLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Minimal => write!(f, "minimal"),
            Self::Standard => write!(f, "standard"),
            Self::Full => write!(f, "full"),
        }
    }
}

impl Display for E2eDiffStrategy {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pixel => write!(f, "pixel"),
            Self::Dom => write!(f, "dom"),
            Self::Hybrid => write!(f, "hybrid"),
            Self::None => write!(f, "none"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_config() -> LoadedE2eConfig {
        let profiles = toml::from_str::<E2eProfilesFile>(
            r#"
            [profile.local-dev]
            backend = "playwright"
            target = "browser"
            scenario_set = "local-smoke"
            browsers = ["chromium"]
            headless = false
            workers = 1
            retries = 0
            trace = "on"
            slow_mo_ms = 250
            mode = "debug"
            default_viewport_set = "responsive-core"
            artifact_level = "full"
            capture_accessibility = true
            capture_dom = true
            capture_layout = true
            capture_console = true
            capture_network = true
            snapshot_diff = "hybrid"
            supported_os = ["macos", "linux", "windows"]
            "#,
        )
        .expect("parse profiles");
        let scenarios = toml::from_str::<E2eScenariosFile>(
            r#"
            [scenario."ui.shell.layout-baseline"]
            summary = "Baseline shell layouts"
            backends = ["playwright"]
            targets = ["browser"]
            tags = ["ui", "shell"]
            slice_family = "shell-layout"
            viewports = ["desktop", "tablet", "mobile"]
            diff_strategy = "hybrid"
            baseline = true
            states = ["default"]
            entry = "shell"
            setup = "shell.layout-baseline"
            assertions = ["selector:[data-ui-kind='desktop-backdrop']"]
            slices = ["shell.soft-neumorphic.default", "shell.modern-adaptive.default"]

            [scenario."ui.shell.navigation-state"]
            summary = "Navigation shell states"
            backends = ["playwright"]
            targets = ["browser"]
            slices = ["shell.soft-neumorphic.context-menu-open"]

            [scenario_set]
            local-smoke = ["ui.shell.layout-baseline", "ui.shell.navigation-state"]
            "#,
        )
        .expect("parse scenarios");

        LoadedE2eConfig {
            profiles,
            scenarios,
        }
    }

    #[test]
    fn parse_defaults_to_list() {
        assert_eq!(parse_e2e_options(&[]).expect("parse"), E2eOptions::List);
    }

    #[test]
    fn parse_doctor_subcommand() {
        assert_eq!(
            parse_e2e_options(&["doctor".into()]).expect("parse"),
            E2eOptions::Doctor
        );
    }

    #[test]
    fn parse_run_subcommand() {
        assert_eq!(
            parse_e2e_options(&[
                "run".into(),
                "--profile".into(),
                "local-dev".into(),
                "--scenario".into(),
                "ui.shell.layout-baseline".into(),
                "--slice".into(),
                "shell.soft-neumorphic.default".into(),
                "--debug".into(),
                "--no-diff".into(),
                "--manifest-out".into(),
                "output/manifest.json".into(),
                "--dry-run".into()
            ])
            .expect("parse"),
            E2eOptions::Run(E2eRunOptions {
                profile: "local-dev".into(),
                scenario: Some("ui.shell.layout-baseline".into()),
                slice: Some("shell.soft-neumorphic.default".into()),
                dry_run: true,
                debug: true,
                no_diff: true,
                base_url: None,
                artifact_dir: None,
                manifest_out: Some(PathBuf::from("output/manifest.json")),
            })
        );
    }

    #[test]
    fn parse_promote_subcommand() {
        assert_eq!(
            parse_e2e_options(&[
                "promote".into(),
                "--profile".into(),
                "local-dev".into(),
                "--scenario".into(),
                "ui.shell.layout-baseline".into(),
                "--slice".into(),
                "shell.soft-neumorphic.default".into(),
                "--source-run".into(),
                "123-local-dev".into(),
            ])
            .expect("parse"),
            E2eOptions::Promote(E2ePromoteOptions {
                profile: "local-dev".into(),
                scenario: Some("ui.shell.layout-baseline".into()),
                slice: Some("shell.soft-neumorphic.default".into()),
                source_run: "123-local-dev".into(),
            })
        );
    }

    #[test]
    fn parse_inspect_subcommand() {
        assert_eq!(
            parse_e2e_options(&["inspect".into(), "--run".into(), "123-local-dev".into()])
                .expect("parse"),
            E2eOptions::Inspect(E2eInspectOptions {
                run: "123-local-dev".into()
            })
        );
    }

    #[test]
    fn parse_run_requires_profile() {
        let err = parse_run_options(&[]).expect_err("profile required");
        assert!(err.to_string().contains("requires `--profile <name>`"));
    }

    #[test]
    fn validate_accepts_fixture_config() {
        validate_e2e_config(&fixture_config()).expect("valid config");
    }

    #[test]
    fn validate_rejects_unknown_trace_policy() {
        let mut config = fixture_config();
        config
            .profiles
            .profile
            .get_mut("local-dev")
            .expect("profile")
            .trace = "bogus".into();
        let err = validate_e2e_config(&config).expect_err("invalid trace");
        assert!(err.to_string().contains("invalid trace policy"));
    }

    #[test]
    fn validate_rejects_unknown_viewport() {
        let mut config = fixture_config();
        config
            .scenarios
            .scenario
            .get_mut("ui.shell.layout-baseline")
            .expect("scenario")
            .viewports
            .push("watch".into());
        let err = validate_e2e_config(&config).expect_err("invalid viewport");
        assert!(err.to_string().contains("unsupported viewport"));
    }

    #[test]
    fn validate_allows_empty_browser_list_for_tauri_webdriver() {
        let profiles = toml::from_str::<E2eProfilesFile>(
            r#"
            [profile.tauri-linux]
            backend = "tauri-webdriver"
            target = "tauri"
            scenario_set = "desktop-smoke"
            browsers = []
            headless = false
            workers = 1
            retries = 0
            trace = "off"
            supported_os = ["linux"]
            "#,
        )
        .expect("parse profiles");
        let scenarios = toml::from_str::<E2eScenariosFile>(
            r#"
            [scenario."desktop.boot"]
            summary = "Boot desktop"
            backends = ["tauri-webdriver"]
            targets = ["tauri"]

            [scenario_set]
            desktop-smoke = ["desktop.boot"]
            "#,
        )
        .expect("parse scenarios");

        validate_e2e_config(&LoadedE2eConfig {
            profiles,
            scenarios,
        })
        .expect("valid desktop config");
    }

    #[test]
    fn resolve_run_uses_default_scenario_set() {
        let resolved = resolve_run(
            &fixture_config(),
            &E2eRunOptions {
                profile: "local-dev".into(),
                scenario: None,
                slice: None,
                dry_run: true,
                debug: false,
                no_diff: false,
                base_url: None,
                artifact_dir: None,
                manifest_out: None,
            },
        )
        .expect("resolve run");

        assert_eq!(
            resolved.scenario_ids,
            vec![
                "ui.shell.layout-baseline".to_string(),
                "ui.shell.navigation-state".to_string()
            ]
        );
    }

    #[test]
    fn resolve_run_rejects_unknown_slice() {
        let err = resolve_run(
            &fixture_config(),
            &E2eRunOptions {
                profile: "local-dev".into(),
                scenario: Some("ui.shell.layout-baseline".into()),
                slice: Some("shell.unknown.default".into()),
                dry_run: true,
                debug: false,
                no_diff: false,
                base_url: None,
                artifact_dir: None,
                manifest_out: None,
            },
        )
        .expect_err("invalid slice");
        assert!(err.to_string().contains("does not define slice"));
    }
}
