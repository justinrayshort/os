//! Top-level CLI parsing and help output.

use crate::runtime::error::{XtaskError, XtaskResult};

/// Top-level `xtask` command families.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TopLevelCommand {
    SetupWeb(Vec<String>),
    Dev(Vec<String>),
    BuildWeb(Vec<String>),
    CheckWeb(Vec<String>),
    Tauri(Vec<String>),
    Flow(Vec<String>),
    Doctor(Vec<String>),
    Docs(Vec<String>),
    Perf(Vec<String>),
    Verify(Vec<String>),
    Help,
}

/// Parse raw command-line arguments into a top-level command selection.
pub fn parse(args: Vec<String>) -> XtaskResult<TopLevelCommand> {
    let Some(cmd) = args.first().cloned() else {
        return Ok(TopLevelCommand::Help);
    };

    let rest = args[1..].to_vec();
    match cmd.as_str() {
        "setup-web" => Ok(TopLevelCommand::SetupWeb(rest)),
        "dev" => Ok(TopLevelCommand::Dev(rest)),
        "build-web" => Ok(TopLevelCommand::BuildWeb(rest)),
        "check-web" => Ok(TopLevelCommand::CheckWeb(rest)),
        "tauri" => Ok(TopLevelCommand::Tauri(rest)),
        "flow" => Ok(TopLevelCommand::Flow(rest)),
        "doctor" => Ok(TopLevelCommand::Doctor(rest)),
        "docs" => Ok(TopLevelCommand::Docs(rest)),
        "perf" => Ok(TopLevelCommand::Perf(rest)),
        "verify" => Ok(TopLevelCommand::Verify(rest)),
        "help" | "--help" | "-h" => Ok(TopLevelCommand::Help),
        other => Err(XtaskError::validation(format!(
            "unknown xtask command: {other}"
        ))),
    }
}

/// Print the canonical top-level usage text.
pub fn print_usage() {
    eprintln!(
        "Usage: cargo xtask <command> [args]\n\
         \n\
         Commands:\n\
           setup-web           Install wasm target and trunk (if missing)\n\
           dev [...]           Prototype dev workflow (serve/start/stop/status/restart/build)\n\
           build-web [args]    Build static web bundle with trunk\n\
           check-web           Run site compile checks (CSR native + wasm)\n\
           tauri [...]         Tauri desktop workflow (dev/build/check)\n\
           flow [...]          Run scoped inner-loop checks for changed packages/docs\n\
           doctor [--fix]      Validate local automation/tooling prerequisites\n\
           docs <subcommand>   Docs validation/audit commands (Rust-native)\n\
           perf <subcommand>   Performance benchmarks/profiling workflows\n\
           verify [fast|full] [--with-desktop|--without-desktop] [--profile <name>]\n\
                              Run standardized local verification workflow (default: full)\n"
    );
}
