use super::DEV_LOOP_BASELINE_DEFAULT_OUTPUT;
use crate::runtime::error::{XtaskError, XtaskResult};
use std::path::PathBuf;

pub(super) fn parse_dev_loop_baseline_output_arg(args: Vec<String>) -> XtaskResult<PathBuf> {
    if args.is_empty() {
        return Ok(PathBuf::from(DEV_LOOP_BASELINE_DEFAULT_OUTPUT));
    }

    if args.len() == 2 && args[0] == "--output" {
        return Ok(PathBuf::from(&args[1]));
    }

    Err(XtaskError::validation(
        "`dev-loop-baseline` expects no args or `--output <path>`",
    ))
}

pub(super) fn parse_named_bench_args(
    command: &str,
    args: Vec<String>,
) -> XtaskResult<(String, Vec<String>)> {
    let Some(baseline) = args.first().cloned() else {
        return Err(XtaskError::validation(format!(
            "`{command}` requires a baseline name"
        )));
    };

    let cargo_args = args[1..].to_vec();
    if cargo_args.iter().any(|arg| arg == "--") {
        return Err(XtaskError::validation(
            "do not pass your own `--`; xtask appends Criterion flags automatically",
        ));
    }

    Ok((baseline, cargo_args))
}

pub(super) fn build_criterion_named_args(
    cargo_args: &[String],
    criterion_flag: &str,
    baseline: &str,
) -> Vec<String> {
    let mut args = if cargo_args.is_empty() {
        vec!["bench".to_string(), "--workspace".to_string()]
    } else {
        let mut user_args = vec!["bench".to_string()];
        user_args.extend(cargo_args.iter().cloned());
        user_args
    };
    args.push("--".to_string());
    args.push(format!("--{criterion_flag}"));
    args.push(baseline.to_string());
    args
}

pub(super) fn flamegraph_args_include_output(args: &[String]) -> bool {
    let mut i = 0usize;
    while i < args.len() {
        let arg = &args[i];
        if arg == "--output" || arg.starts_with("--output=") {
            return true;
        }
        i += 1;
    }
    false
}

pub(super) fn is_sccache_wrapper(wrapper: &str) -> bool {
    wrapper == "sccache" || wrapper.ends_with("/sccache")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn criterion_args_default_to_workspace() {
        let args = build_criterion_named_args(&[], "save-baseline", "local-main");
        assert_eq!(
            args,
            vec![
                "bench",
                "--workspace",
                "--",
                "--save-baseline",
                "local-main",
            ]
        );
    }

    #[test]
    fn criterion_args_preserve_cargo_filters() {
        let args = build_criterion_named_args(
            &["--bench".into(), "runtime".into()],
            "baseline",
            "local-main",
        );
        assert_eq!(
            args,
            vec![
                "bench",
                "--bench",
                "runtime",
                "--",
                "--baseline",
                "local-main"
            ]
        );
    }

    #[test]
    fn named_bench_args_reject_user_double_dash() {
        let err = parse_named_bench_args("baseline", vec!["main".into(), "--".into()]).unwrap_err();
        assert!(err.to_string().contains("do not pass your own"));
    }

    #[test]
    fn dev_loop_baseline_output_arg_defaults_to_standard_path() {
        let output = parse_dev_loop_baseline_output_arg(Vec::new()).expect("parse");
        assert_eq!(output, PathBuf::from(DEV_LOOP_BASELINE_DEFAULT_OUTPUT));
    }

    #[test]
    fn dev_loop_baseline_output_arg_accepts_explicit_path() {
        let output = parse_dev_loop_baseline_output_arg(vec![
            "--output".into(),
            ".artifacts/custom.json".into(),
        ])
        .expect("parse");
        assert_eq!(output, PathBuf::from(".artifacts/custom.json"));
    }

    #[test]
    fn dev_loop_baseline_output_arg_rejects_invalid_shape() {
        let err = parse_dev_loop_baseline_output_arg(vec!["oops".into()]).unwrap_err();
        assert!(err.to_string().contains("expects no args"));
    }

    #[test]
    fn flamegraph_output_flag_detection_handles_short_and_long_forms() {
        assert!(flamegraph_args_include_output(&[
            "--output".into(),
            "a.svg".into()
        ]));
        assert!(flamegraph_args_include_output(&["--output=a.svg".into()]));
        assert!(!flamegraph_args_include_output(&[
            "--bench".into(),
            "foo".into()
        ]));
    }

    #[test]
    fn sccache_wrapper_detection_handles_binary_and_path() {
        assert!(is_sccache_wrapper("sccache"));
        assert!(is_sccache_wrapper("/usr/local/bin/sccache"));
        assert!(!is_sccache_wrapper("/usr/local/bin/clang"));
    }
}
