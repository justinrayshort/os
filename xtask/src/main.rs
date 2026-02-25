use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

fn main() -> ExitCode {
    let root = workspace_root();
    let mut args = env::args().skip(1);

    let Some(cmd) = args.next() else {
        print_usage();
        return ExitCode::from(2);
    };

    let rest: Vec<String> = args.collect();

    let result = match cmd.as_str() {
        "setup-web" => setup_web(&root),
        "dev" => dev_server(&root, rest),
        "build-web" => build_web(&root, rest),
        "check-web" => check_web(&root),
        "verify" => verify(&root, rest),
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => Err(format!("unknown xtask command: {other}")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives under workspace root")
        .to_path_buf()
}

fn print_usage() {
    eprintln!(
        "Usage: cargo xtask <command> [args]\n\
         \n\
         Commands:\n\
           setup-web           Install wasm target and trunk (if missing)\n\
           dev [trunk args]    Start trunk dev server (defaults to --open)\n\
           build-web [args]    Build static web bundle with trunk\n\
           check-web           Run site compile checks (hydrate/ssr/wasm)\n\
           verify [fast|full]  Run scripts/ci/verify.sh (default: full)\n"
    );
}

fn setup_web(root: &Path) -> Result<(), String> {
    run(
        root,
        "rustup",
        vec!["target", "add", "wasm32-unknown-unknown"],
    )?;

    if command_available("trunk") {
        println!("trunk already installed");
        return Ok(());
    }

    run(root, "cargo", vec!["install", "trunk"])
}

fn dev_server(root: &Path, args: Vec<String>) -> Result<(), String> {
    ensure_command(
        "trunk",
        "Install it with `cargo setup-web` (or `cargo install trunk`)",
    )?;

    let mut pass_through = Vec::new();
    let mut open = true;
    for arg in args {
        if arg == "--no-open" {
            open = false;
        } else {
            pass_through.push(arg);
        }
    }

    let mut trunk_args = vec![
        "serve".to_string(),
        "crates/site/index.html".to_string(),
        "--features".to_string(),
        "hydrate".to_string(),
    ];
    if open {
        trunk_args.push("--open".to_string());
    }
    trunk_args.extend(pass_through);

    run_owned(root, "trunk", trunk_args)
}

fn build_web(root: &Path, args: Vec<String>) -> Result<(), String> {
    ensure_command(
        "trunk",
        "Install it with `cargo setup-web` (or `cargo install trunk`)",
    )?;

    let mut trunk_args = vec![
        "build".to_string(),
        "crates/site/index.html".to_string(),
        "--features".to_string(),
        "hydrate".to_string(),
        "--release".to_string(),
        "--dist".to_string(),
        "target/trunk-dist".to_string(),
    ];
    trunk_args.extend(args);

    run_owned(root, "trunk", trunk_args)
}

fn check_web(root: &Path) -> Result<(), String> {
    run(
        root,
        "cargo",
        vec!["check", "-p", "site", "--features", "hydrate"],
    )?;
    run(
        root,
        "cargo",
        vec!["check", "-p", "site", "--features", "ssr"],
    )?;

    if wasm_target_installed() {
        run(
            root,
            "cargo",
            vec![
                "check",
                "-p",
                "site",
                "--target",
                "wasm32-unknown-unknown",
                "--features",
                "hydrate",
            ],
        )?;
    } else {
        eprintln!(
            "warn: wasm32-unknown-unknown target not installed; skipping wasm check (run `cargo setup-web`)"
        );
    }

    Ok(())
}

fn verify(root: &Path, args: Vec<String>) -> Result<(), String> {
    let mode = args.first().map(String::as_str).unwrap_or("full");
    match mode {
        "fast" | "full" => run_owned(root, "./scripts/ci/verify.sh", vec![mode.to_string()]),
        _ => Err(format!(
            "invalid verify mode `{mode}` (expected `fast` or `full`)"
        )),
    }
}

fn wasm_target_installed() -> bool {
    let Ok(output) = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
    else {
        return false;
    };

    if !output.status.success() {
        return false;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.trim() == "wasm32-unknown-unknown")
}

fn command_available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn ensure_command(program: &str, hint: &str) -> Result<(), String> {
    if command_available(program) {
        Ok(())
    } else {
        Err(format!("required command `{program}` not found. {hint}"))
    }
}

fn run(root: &Path, program: &str, args: Vec<&str>) -> Result<(), String> {
    let owned = args.into_iter().map(ToString::to_string).collect();
    run_owned(root, program, owned)
}

fn run_owned(root: &Path, program: &str, args: Vec<String>) -> Result<(), String> {
    print_command(program, &args);
    let status = Command::new(program)
        .current_dir(root)
        .args(&args)
        .status()
        .map_err(|err| format!("failed to start `{program}`: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("`{program}` exited with status {status}"))
    }
}

fn print_command(program: &str, args: &[String]) {
    if args.is_empty() {
        println!("+ {program}");
        return;
    }

    println!("+ {program} {}", args.join(" "));
}
