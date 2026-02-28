use crate::runtime::config::ConfigLoader;
use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use serde::Deserialize;
use std::collections::BTreeMap;

pub(super) const VERIFY_PROFILES_FILE: &str = "tools/automation/verify_profiles.toml";

#[derive(Clone, Debug, Deserialize)]
struct VerifyProfilesFile {
    profile: BTreeMap<String, VerifyProfileSpec>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct VerifyProfileSpec {
    pub(super) mode: String,
    pub(super) desktop_mode: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VerifyMode {
    Fast,
    Full,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VerifyFastDesktopMode {
    Auto,
    WithDesktop,
    WithoutDesktop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifyOptions {
    pub(crate) mode: VerifyMode,
    pub(crate) desktop_mode: VerifyFastDesktopMode,
    pub(crate) explicit_mode: bool,
    pub(crate) explicit_desktop_mode: bool,
    pub(crate) profile: Option<String>,
    pub(crate) show_help: bool,
}

pub(super) fn load_verify_profiles(
    ctx: &CommandContext,
) -> XtaskResult<BTreeMap<String, VerifyProfileSpec>> {
    let loader = ConfigLoader::<VerifyProfilesFile>::new(ctx.root(), VERIFY_PROFILES_FILE);
    let parsed = loader.load()?;
    if parsed.profile.is_empty() {
        return Err(XtaskError::config(format!(
            "{} does not define any profiles",
            loader.path().display()
        )));
    }
    Ok(parsed.profile)
}

fn resolve_verify_profile(
    profile_name: &str,
    profiles: &BTreeMap<String, VerifyProfileSpec>,
) -> XtaskResult<(VerifyMode, VerifyFastDesktopMode)> {
    let Some(profile) = profiles.get(profile_name) else {
        let known = profiles.keys().cloned().collect::<Vec<_>>().join(", ");
        return Err(XtaskError::config(format!(
            "unknown verify profile `{profile_name}` (known: {known})"
        )));
    };

    let mode = match profile.mode.as_str() {
        "fast" => VerifyMode::Fast,
        "full" => VerifyMode::Full,
        other => {
            return Err(XtaskError::config(format!(
                "verify profile `{profile_name}` has invalid mode `{other}` (expected `fast` or `full`)"
            )))
        }
    };

    let desktop_mode = match profile.desktop_mode.as_deref().unwrap_or("auto") {
        "auto" => VerifyFastDesktopMode::Auto,
        "with-desktop" => VerifyFastDesktopMode::WithDesktop,
        "without-desktop" => VerifyFastDesktopMode::WithoutDesktop,
        other => {
            return Err(XtaskError::config(format!(
                "verify profile `{profile_name}` has invalid desktop_mode `{other}` (expected `auto`, `with-desktop`, `without-desktop`)"
            )))
        }
    };

    Ok((mode, desktop_mode))
}

fn print_verify_profile_selection(
    name: &str,
    mode: VerifyMode,
    desktop_mode: VerifyFastDesktopMode,
) {
    let mode_text = match mode {
        VerifyMode::Fast => "fast",
        VerifyMode::Full => "full",
    };
    let desktop_text = match desktop_mode {
        VerifyFastDesktopMode::Auto => "auto",
        VerifyFastDesktopMode::WithDesktop => "with-desktop",
        VerifyFastDesktopMode::WithoutDesktop => "without-desktop",
    };
    println!(
        "\n==> Verify profile selected: `{name}` (mode={mode_text}, desktop_mode={desktop_text})"
    );
}

pub(super) fn resolve_verify_options_from_profile(
    mut options: VerifyOptions,
    profiles: &BTreeMap<String, VerifyProfileSpec>,
) -> XtaskResult<VerifyOptions> {
    let Some(profile_name) = options.profile.clone() else {
        return Ok(options);
    };
    if options.explicit_mode {
        return Err(XtaskError::validation(
            "`--profile` cannot be combined with `fast`/`full` positional mode",
        ));
    }
    if options.explicit_desktop_mode {
        return Err(XtaskError::validation(
            "`--profile` cannot be combined with `--with-desktop`/`--without-desktop`",
        ));
    }
    let (mode, desktop_mode) = resolve_verify_profile(&profile_name, profiles)?;
    options.mode = mode;
    options.desktop_mode = desktop_mode;
    print_verify_profile_selection(&profile_name, mode, desktop_mode);
    Ok(options)
}

fn verify_profile_names(profiles: &BTreeMap<String, VerifyProfileSpec>) -> String {
    profiles.keys().cloned().collect::<Vec<_>>().join(", ")
}

pub(super) fn print_verify_usage(profiles: Option<&BTreeMap<String, VerifyProfileSpec>>) {
    let profile_list = profiles
        .map(verify_profile_names)
        .unwrap_or_else(|| "<unavailable>".to_string());
    eprintln!(
        "Usage: cargo verify [fast|full] [--with-desktop|--without-desktop] [--profile <name>]\n\
         \n\
         Profiles:\n\
           {}\n\
         \n\
         Notes:\n\
           - `--profile` cannot be combined with explicit `fast`/`full` or desktop flags.\n\
           - desktop flags are only valid with `fast` mode.\n",
        profile_list
    );
}

pub(super) fn parse_verify_options(args: Vec<String>) -> XtaskResult<VerifyOptions> {
    let mut options = VerifyOptions {
        mode: VerifyMode::Full,
        desktop_mode: VerifyFastDesktopMode::Auto,
        explicit_mode: false,
        explicit_desktop_mode: false,
        profile: None,
        show_help: false,
    };
    let mut i = 0usize;

    if let Some(first) = args.first().map(String::as_str) {
        match first {
            "fast" => {
                options.mode = VerifyMode::Fast;
                options.explicit_mode = true;
                i = 1;
            }
            "full" => {
                options.mode = VerifyMode::Full;
                options.explicit_mode = true;
                i = 1;
            }
            _ => {}
        }
    }

    while i < args.len() {
        match args[i].as_str() {
            "--with-desktop" => {
                if options.desktop_mode == VerifyFastDesktopMode::WithoutDesktop {
                    return Err(XtaskError::validation(
                        "`--with-desktop` cannot be combined with `--without-desktop`",
                    ));
                }
                options.desktop_mode = VerifyFastDesktopMode::WithDesktop;
                options.explicit_desktop_mode = true;
                i += 1;
            }
            "--without-desktop" => {
                if options.desktop_mode == VerifyFastDesktopMode::WithDesktop {
                    return Err(XtaskError::validation(
                        "`--with-desktop` cannot be combined with `--without-desktop`",
                    ));
                }
                options.desktop_mode = VerifyFastDesktopMode::WithoutDesktop;
                options.explicit_desktop_mode = true;
                i += 1;
            }
            "--profile" => {
                let Some(profile) = args.get(i + 1) else {
                    return Err(XtaskError::validation("missing value for `--profile`"));
                };
                options.profile = Some(profile.clone());
                i += 2;
            }
            "help" | "--help" | "-h" => {
                options.show_help = true;
                i += 1;
            }
            other => {
                return Err(XtaskError::validation(format!(
                    "unsupported `cargo verify` argument `{other}` (expected `fast`, `full`, `--with-desktop`, `--without-desktop`, `--profile`)"
                )));
            }
        }
    }

    Ok(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_profiles() -> BTreeMap<String, VerifyProfileSpec> {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "dev".to_string(),
            VerifyProfileSpec {
                mode: "fast".into(),
                desktop_mode: Some("auto".into()),
            },
        );
        profiles.insert(
            "release".to_string(),
            VerifyProfileSpec {
                mode: "full".into(),
                desktop_mode: Some("auto".into()),
            },
        );
        profiles
    }

    #[test]
    fn verify_option_parser_defaults_to_full_mode() {
        let parsed = parse_verify_options(Vec::new()).expect("parse");
        assert_eq!(parsed.mode, VerifyMode::Full);
        assert_eq!(parsed.desktop_mode, VerifyFastDesktopMode::Auto);
    }

    #[test]
    fn verify_option_parser_accepts_fast_desktop_flags() {
        let parsed =
            parse_verify_options(vec!["fast".into(), "--with-desktop".into()]).expect("parse");
        assert_eq!(parsed.mode, VerifyMode::Fast);
        assert_eq!(parsed.desktop_mode, VerifyFastDesktopMode::WithDesktop);
    }

    #[test]
    fn verify_option_parser_rejects_conflicting_desktop_flags() {
        let err = parse_verify_options(vec![
            "fast".into(),
            "--with-desktop".into(),
            "--without-desktop".into(),
        ])
        .unwrap_err();
        assert!(err.to_string().contains("cannot be combined"));
    }

    #[test]
    fn verify_option_parser_supports_profiles() {
        let parsed =
            parse_verify_options(vec!["--profile".into(), "dev".into()]).expect("parse options");
        let options =
            resolve_verify_options_from_profile(parsed, &test_profiles()).expect("resolve profile");
        assert_eq!(options.mode, VerifyMode::Fast);
        assert_eq!(options.profile.as_deref(), Some("dev"));
    }
}
