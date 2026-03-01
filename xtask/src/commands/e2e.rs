//! Cargo-managed end-to-end workflow foundation.

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
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};
use std::time::Duration;

const E2E_PROFILES_CONFIG: &str = "tools/automation/e2e_profiles.toml";
const E2E_SCENARIOS_CONFIG: &str = "tools/automation/e2e_scenarios.toml";
const E2E_NODE_PACKAGE: &str = "tools/e2e/package.json";
const E2E_NODE_LOCKFILE: &str = "tools/e2e/package-lock.json";
const E2E_NODE_MODULES: &str = "tools/e2e/node_modules";
const E2E_PROJECT_DIR: &str = "tools/e2e";
const DESKTOP_TAURI_PACKAGE: &str = "desktop_tauri";

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
        let browsers = if profile.browsers.is_empty() {
            "(n/a)".to_string()
        } else {
            profile.browsers.join(",")
        };
        println!(
            "  - {name}: backend={}, target={}, scenario_set={}, browsers={}, headless={}, workers={}, retries={}, trace={}",
            profile.backend,
            profile.target,
            profile.scenario_set,
            browsers,
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

    let current_os = SupportedOs::current();
    let node_available = ctx.process().command_available("node");
    let npm_available = ctx.process().command_available("npm");
    let trunk_available = ctx.process().command_available("trunk");
    let cargo_available = ctx.process().command_available("cargo");
    let tauri_driver_available = ctx.process().command_available("tauri-driver");
    let native_webdriver_available = ["geckodriver", "chromedriver", "msedgedriver"]
        .iter()
        .any(|program| ctx.process().command_available(program));
    let package_json = ctx.artifacts().path(E2E_NODE_PACKAGE);
    let lockfile = ctx.artifacts().path(E2E_NODE_LOCKFILE);
    let node_modules = ctx.artifacts().path(E2E_NODE_MODULES);
    let desktop_profiles = config
        .profiles
        .profile
        .iter()
        .filter(|(_, profile)| profile.backend == E2eBackend::TauriWebdriver)
        .map(|(name, _)| name.as_str())
        .collect::<Vec<_>>();
    let desktop_supported_here = desktop_profiles.iter().any(|name| {
        config
            .profiles
            .profile
            .get(*name)
            .map(|profile| profile.supported_os.contains(&current_os))
            .unwrap_or(false)
    });

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
        "  - desktop backend support on {}: {}",
        current_os,
        if desktop_supported_here {
            "supported"
        } else {
            "not supported"
        }
    );
    println!("  - tauri-driver: {}", status_word(tauri_driver_available));
    println!(
        "  - native webdriver bridge: {}",
        status_word(native_webdriver_available)
    );
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
                println!(
                    "browsers: {}",
                    if resolved.profile.browsers.is_empty() {
                        "(n/a)".to_string()
                    } else {
                        resolved.profile.browsers.join(", ")
                    }
                );
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

pub(crate) fn run_named_profile(ctx: &CommandContext, profile: &str) -> XtaskResult<()> {
    e2e_run(
        ctx,
        E2eRunOptions {
            profile: profile.to_string(),
            scenario: None,
            dry_run: false,
            base_url: None,
            artifact_dir: None,
        },
    )
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
                .run_timed_stage("Bootstrap E2E Node dependencies", || {
                    ensure_node_modules(ctx)
                })?;
            let dev_config = load_dev_server_config(ctx)?;
            let desktop_binary = build_desktop_tauri_binary(ctx)?;
            let native_driver = resolve_native_webdriver_binary(ctx)?;
            let mut frontend_server = None;
            ctx.workflow()
                .run_timed_stage("Start desktop E2E frontend server", || {
                    frontend_server = Some(start_isolated_e2e_server(ctx, artifact_dir, &dev_config)?);
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
    ctx.workflow()
        .run_timed_stage("Execute Playwright harness", || {
            let scenario_ids = resolved.scenario_ids.join(",");
            let browsers = resolved.profile.browsers.join(",");
            let headless = if resolved.profile.headless {
                "true"
            } else {
                "false"
            };
            let retries = resolved.profile.retries.to_string();
            let slow_mo_ms = resolved.profile.slow_mo_ms.unwrap_or(0).to_string();
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
                    ("OS_E2E_RETRIES", retries.as_str()),
                    ("OS_E2E_SLOW_MO_MS", slow_mo_ms.as_str()),
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
            slow_mo_ms = 250
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
