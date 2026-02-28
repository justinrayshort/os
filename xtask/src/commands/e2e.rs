//! Cargo-managed end-to-end workflow foundation.

use crate::commands::dev::{load_dev_server_config, DevServerConfig};
use crate::runtime::config::ConfigLoader;
use crate::runtime::context::CommandContext;
use crate::runtime::env::EnvHelper;
use crate::runtime::error::{XtaskError, XtaskResult};
use crate::runtime::fs::read_file_tail;
use crate::runtime::lifecycle::{kill_pid, port_is_open, terminate_pid};
use crate::runtime::workflow::unix_timestamp_millis;
use crate::XtaskCommand;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::fs::OpenOptions;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const E2E_PROFILES_CONFIG: &str = "tools/automation/e2e_profiles.toml";
const E2E_SCENARIOS_CONFIG: &str = "tools/automation/e2e_scenarios.toml";
const E2E_NODE_PACKAGE: &str = "tools/e2e/package.json";
const E2E_NODE_LOCKFILE: &str = "tools/e2e/package-lock.json";
const E2E_NODE_MODULES: &str = "tools/e2e/node_modules";
const E2E_PROJECT_DIR: &str = "tools/e2e";

/// `cargo e2e ...`
pub struct E2eCommand;

/// Supported `cargo e2e` subcommands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum E2eOptions {
    List,
    Doctor,
    Run(E2eRunOptions),
    Help,
}

/// Typed `cargo e2e run` options.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct E2eRunOptions {
    pub profile: String,
    pub scenario: Option<String>,
    pub dry_run: bool,
    pub base_url: Option<String>,
    pub artifact_dir: Option<PathBuf>,
}

#[derive(Debug)]
struct E2eServerHandle {
    child: Child,
    host: String,
    port: u16,
    log_path: PathBuf,
}

impl E2eServerHandle {
    fn pid(&self) -> u32 {
        self.child.id()
    }

    fn base_url(&self) -> String {
        format!("http://{}:{}/", self.host, self.port)
    }
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
    supported_os: Vec<SupportedOs>,
}

#[derive(Clone, Debug, Deserialize)]
struct E2eScenarioSpec {
    summary: String,
    backends: Vec<E2eBackend>,
    targets: Vec<E2eTarget>,
    tags: Option<Vec<String>>,
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
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "kebab-case")]
enum E2eBackend {
    Playwright,
    TauriWebdriver,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
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
        Some("help" | "--help" | "-h") => Ok(E2eOptions::Help),
        Some(other) => Err(XtaskError::validation(format!(
            "unknown e2e subcommand: {other}"
        ))),
    }
}

fn parse_run_options(args: &[String]) -> XtaskResult<E2eRunOptions> {
    let mut profile = None;
    let mut scenario = None;
    let mut dry_run = false;
    let mut base_url = None;
    let mut artifact_dir = None;
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
            "--dry-run" => {
                dry_run = true;
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
        dry_run,
        base_url,
        artifact_dir,
    })
}

impl E2eRunOptions {
    fn help_defaults() -> Self {
        Self {
            profile: String::new(),
            scenario: None,
            dry_run: false,
            base_url: None,
            artifact_dir: None,
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
           run --profile <name> [--scenario <id>] [--base-url <url>] [--artifact-dir <path>] [--dry-run]\n\
                                 Resolve a profile/scenario selection and execute the Playwright harness\n"
    );
}

fn e2e_list(ctx: &CommandContext) -> XtaskResult<()> {
    let config = load_e2e_config(ctx)?;
    validate_e2e_config(&config)?;

    println!("E2E profiles:");
    for (name, profile) in &config.profiles.profile {
        println!(
            "  - {name}: backend={}, target={}, scenario_set={}, browsers={}, headless={}, workers={}, retries={}, trace={}",
            profile.backend,
            profile.target,
            profile.scenario_set,
            profile.browsers.join(","),
            profile.headless,
            profile.workers,
            profile.retries,
            profile.trace
        );
    }

    println!("\nE2E scenario sets:");
    for (set_name, scenario_ids) in &config.scenarios.scenario_set {
        println!("  - {set_name}: {}", scenario_ids.join(", "));
    }

    println!("\nE2E scenarios:");
    for (scenario_id, scenario) in &config.scenarios.scenario {
        let tags = scenario
            .tags
            .as_ref()
            .map(|tags| tags.join(","))
            .unwrap_or_else(|| "none".into());
        println!(
            "  - {scenario_id}: summary={}, backends={}, targets={}, tags={}",
            scenario.summary,
            scenario
                .backends
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
            scenario
                .targets
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
            tags
        );
    }

    Ok(())
}

fn e2e_doctor(ctx: &CommandContext) -> XtaskResult<()> {
    let config = load_e2e_config(ctx)?;
    validate_e2e_config(&config)?;

    let node_available = ctx.process().command_available("node");
    let npm_available = ctx.process().command_available("npm");
    let trunk_available = ctx.process().command_available("trunk");
    let cargo_available = ctx.process().command_available("cargo");
    let package_json = ctx.artifacts().path(E2E_NODE_PACKAGE);
    let lockfile = ctx.artifacts().path(E2E_NODE_LOCKFILE);
    let node_modules = ctx.artifacts().path(E2E_NODE_MODULES);

    println!("E2E doctor:");
    println!(
        "  - config profiles: {} ({})",
        config.profiles.profile.len(),
        ctx.root().join(E2E_PROFILES_CONFIG).display()
    );
    println!(
        "  - config scenarios: {} ({})",
        config.scenarios.scenario.len(),
        ctx.root().join(E2E_SCENARIOS_CONFIG).display()
    );
    println!("  - cargo: {}", status_word(cargo_available));
    println!("  - node: {}", status_word(node_available));
    println!("  - npm: {}", status_word(npm_available));
    println!("  - trunk: {}", status_word(trunk_available));
    println!(
        "  - tools/e2e package: {}",
        if package_json.exists() {
            "present"
        } else {
            "missing"
        }
    );
    println!(
        "  - tools/e2e lockfile: {}",
        if lockfile.exists() {
            "present"
        } else {
            "pending bootstrap"
        }
    );
    println!(
        "  - tools/e2e node_modules: {}",
        if node_modules.exists() {
            "ready"
        } else {
            "pending npm ci"
        }
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

    if failures.is_empty() {
        if !node_modules.exists() {
            ctx.workflow().warn(
                "tools/e2e/node_modules is not present yet; `cargo e2e run` will bootstrap dependencies with `npm ci`",
            );
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
                let planned_run_dir = planned_e2e_run_dir(ctx, &resolved, &options);
                let node_lockfile = ctx.artifacts().path(E2E_NODE_LOCKFILE);
                let base_url = planned_base_url(ctx, &options)?;

                println!("profile: {}", resolved.profile_name);
                println!("backend: {}", resolved.profile.backend);
                println!("target: {}", resolved.profile.target);
                println!("scenario set: {}", resolved.profile.scenario_set);
                println!("scenario ids: {}", resolved.scenario_ids.join(", "));
                println!("browsers: {}", resolved.profile.browsers.join(", "));
                println!("headless: {}", resolved.profile.headless);
                println!("workers: {}", resolved.profile.workers);
                println!("trace: {}", resolved.profile.trace);
                println!("base url: {base_url}");
                println!("artifact root: {}", planned_run_dir.display());
                println!("node lockfile: {}", node_lockfile.display());

                if options.dry_run {
                    println!("mode: dry-run");
                    println!("status: configuration resolved successfully; no browser automation executed");
                    Ok(())
                } else {
                    run_e2e_backend(ctx, &resolved, &options, &planned_run_dir)
                }
            })
        })
}

fn run_e2e_backend(
    ctx: &CommandContext,
    resolved: &ResolvedE2eRun,
    options: &E2eRunOptions,
    artifact_dir: &Path,
) -> XtaskResult<()> {
    match resolved.profile.backend {
        E2eBackend::Playwright => run_playwright_backend(ctx, resolved, options, artifact_dir),
        E2eBackend::TauriWebdriver => Err(XtaskError::unsupported_platform(
            "tauri-webdriver execution is not implemented yet; use a browser profile for now",
        )),
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
    for (profile_name, profile) in &config.profiles.profile {
        let Some(scenarios) = config.scenarios.scenario_set.get(&profile.scenario_set) else {
            return Err(XtaskError::config(format!(
                "E2E profile `{profile_name}` references unknown scenario set `{}`",
                profile.scenario_set
            )));
        };

        if profile.browsers.is_empty() {
            return Err(XtaskError::config(format!(
                "E2E profile `{profile_name}` must define at least one browser"
            )));
        }

        if profile.supported_os.is_empty() {
            return Err(XtaskError::config(format!(
                "E2E profile `{profile_name}` must define at least one supported OS"
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
    })
}

fn ensure_supported_os(profile: &E2eProfileSpec, profile_name: &str) -> XtaskResult<()> {
    let current = SupportedOs::current();
    if profile.supported_os.contains(&current) {
        Ok(())
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

fn planned_base_url(ctx: &CommandContext, options: &E2eRunOptions) -> XtaskResult<String> {
    if let Some(base_url) = &options.base_url {
        return Ok(base_url.clone());
    }

    let config = load_dev_server_config(ctx)?;
    let host = normalized_host_for_url(&config.default_host);
    Ok(format!("http://{host}:<ephemeral>/"))
}

fn normalized_host_for_url(host: &str) -> String {
    match host {
        "0.0.0.0" => "127.0.0.1".to_string(),
        "::" => "[::1]".to_string(),
        other => other.to_string(),
    }
}

fn allocate_local_port(host: &str) -> XtaskResult<u16> {
    let bind_addr = if host == "::" {
        "[::1]:0"
    } else {
        "127.0.0.1:0"
    };
    let listener = TcpListener::bind(bind_addr).map_err(|err| {
        XtaskError::io(format!(
            "failed to allocate an ephemeral E2E port on {bind_addr}: {err}"
        ))
    })?;
    let port = listener
        .local_addr()
        .map_err(|err| XtaskError::io(format!("failed to inspect allocated port: {err}")))?
        .port();
    drop(listener);
    Ok(port)
}

fn start_isolated_e2e_server(
    ctx: &CommandContext,
    artifact_dir: &Path,
    config: &DevServerConfig,
) -> XtaskResult<E2eServerHandle> {
    let host = config.default_host.clone();
    let public_host = normalized_host_for_url(&host);
    let port = allocate_local_port(&host)?;
    let server_dir = artifact_dir.join("server");
    ctx.artifacts().ensure_dir(&server_dir)?;
    let log_path = server_dir.join("trunk.log");

    let log = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&log_path)
        .map_err(|err| XtaskError::io(format!("failed to open {}: {err}", log_path.display())))?;
    let log_out = log
        .try_clone()
        .map_err(|err| XtaskError::io(format!("failed to clone E2E log handle: {err}")))?;

    let args = vec![
        "serve".to_string(),
        "index.html".to_string(),
        "--no-sri=true".to_string(),
        "--ignore".to_string(),
        "dist".to_string(),
        "--address".to_string(),
        host.clone(),
        "--port".to_string(),
        port.to_string(),
    ];
    ctx.process().print_command("trunk", &args);

    let mut cmd = Command::new("trunk");
    cmd.current_dir(ctx.root().join("crates/site"))
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log_out))
        .stderr(Stdio::from(log));
    EnvHelper.apply_no_color_override(&mut cmd);

    let child = cmd.spawn().map_err(|err| {
        XtaskError::process_launch(format!("failed to start isolated `trunk`: {err}"))
    })?;

    let mut state = E2eServerHandle {
        child,
        host: public_host,
        port,
        log_path,
    };

    wait_for_e2e_server_startup(
        &mut state.child,
        &state.log_path,
        &state.host,
        state.port,
        config.start_poll(),
    )?;
    Ok(state)
}

fn wait_for_e2e_server_startup(
    child: &mut Child,
    log_path: &Path,
    host: &str,
    port: u16,
    timeout: Duration,
) -> XtaskResult<()> {
    let deadline = Instant::now() + timeout;

    loop {
        if port_is_open(host, port) {
            return Ok(());
        }

        if let Some(status) = child.try_wait().map_err(|err| {
            XtaskError::process_exit(format!("failed while checking E2E server startup: {err}"))
        })? {
            let mut msg =
                format!("isolated E2E dev server exited during startup with status {status}");
            let tail = read_file_tail(log_path, 20).unwrap_or_default();
            if !tail.is_empty() {
                msg.push_str(&format!("\nlog tail ({}):\n{}", log_path.display(), tail));
            }
            return Err(XtaskError::process_exit(msg));
        }

        if Instant::now() >= deadline {
            let tail = read_file_tail(log_path, 20).unwrap_or_default();
            return Err(XtaskError::process_exit(format!(
                "isolated E2E dev server did not become reachable at {} within {:?}\nlog tail ({}):\n{}",
                format_args!("http://{host}:{port}/"),
                timeout,
                log_path.display(),
                tail
            )));
        }

        thread::sleep(Duration::from_millis(200));
    }
}

fn stop_isolated_e2e_server(ctx: &CommandContext, state: &mut E2eServerHandle) -> XtaskResult<()> {
    ctx.workflow()
        .run_timed_stage("Stop isolated E2E dev server", || {
            let pid = state.pid();
            terminate_pid(pid)?;
            let deadline = Instant::now() + Duration::from_secs(5);
            while Instant::now() < deadline {
                if let Some(status) = state.child.try_wait().map_err(|err| {
                    XtaskError::process_exit(format!(
                        "failed while waiting for isolated E2E dev server shutdown: {err}"
                    ))
                })? {
                    if status.success() || !port_is_open(&state.host, state.port) {
                        return Ok(());
                    }
                    return Err(XtaskError::process_exit(format!(
                        "isolated E2E dev server exited with status {status}"
                    )));
                }
                if !port_is_open(&state.host, state.port) {
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(100));
            }
            kill_pid(pid)?;
            let status = state.child.wait().map_err(|err| {
                XtaskError::process_exit(format!(
                    "failed to reap isolated E2E dev server process {pid}: {err}"
                ))
            })?;
            if status.success() || !port_is_open(&state.host, state.port) {
                Ok(())
            } else {
                Err(XtaskError::process_exit(format!(
                    "failed to stop isolated E2E dev server pid {pid} cleanly (status {status})"
                )))
            }
        })
}

fn execute_playwright_harness(
    ctx: &CommandContext,
    resolved: &ResolvedE2eRun,
    artifact_dir: &Path,
    base_url: &str,
) -> XtaskResult<()> {
    ctx.workflow()
        .run_timed_stage("Execute Playwright harness", || {
            let scenario_ids = resolved.scenario_ids.join(",");
            let browsers = resolved.profile.browsers.join(",");
            let headless = if resolved.profile.headless {
                "true"
            } else {
                "false"
            };
            let artifact_dir_string = artifact_dir.display().to_string();

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
                    ("OS_E2E_SCENARIO_IDS", scenario_ids.as_str()),
                    ("OS_E2E_BROWSERS", browsers.as_str()),
                    ("OS_E2E_HEADLESS", headless),
                    ("OS_E2E_TRACE", resolved.profile.trace.as_str()),
                ],
            )
        })
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
            supported_os = ["macos", "linux", "windows"]
            "#,
        )
        .expect("parse profiles");
        let scenarios = toml::from_str::<E2eScenariosFile>(
            r#"
            [scenario."shell.boot"]
            summary = "Boot shell"
            backends = ["playwright"]
            targets = ["browser"]
            tags = ["smoke"]

            [scenario."shell.settings-navigation"]
            summary = "Navigate settings"
            backends = ["playwright"]
            targets = ["browser"]

            [scenario_set]
            local-smoke = ["shell.boot", "shell.settings-navigation"]
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
                "shell.boot".into(),
                "--dry-run".into()
            ])
            .expect("parse"),
            E2eOptions::Run(E2eRunOptions {
                profile: "local-dev".into(),
                scenario: Some("shell.boot".into()),
                dry_run: true,
                base_url: None,
                artifact_dir: None,
            })
        );
    }

    #[test]
    fn parse_run_requires_profile() {
        let err = parse_run_options(&[]).expect_err("profile required");
        assert!(err.to_string().contains("requires `--profile <name>`"));
    }

    #[test]
    fn parse_rejects_unknown_subcommand() {
        let err = parse_e2e_options(&["bogus".into()]).expect_err("invalid");
        assert!(err.to_string().contains("unknown e2e subcommand"));
    }

    #[test]
    fn validate_accepts_fixture_config() {
        validate_e2e_config(&fixture_config()).expect("valid config");
    }

    #[test]
    fn resolve_run_uses_default_scenario_set() {
        let resolved = resolve_run(
            &fixture_config(),
            &E2eRunOptions {
                profile: "local-dev".into(),
                scenario: None,
                dry_run: true,
                base_url: None,
                artifact_dir: None,
            },
        )
        .expect("resolve run");

        assert_eq!(
            resolved.scenario_ids,
            vec![
                "shell.boot".to_string(),
                "shell.settings-navigation".to_string()
            ]
        );
    }
}
