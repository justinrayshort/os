use super::config::VERIFY_PROFILES_FILE;
use crate::runtime::context::CommandContext;
use crate::runtime::error::XtaskResult;
use crate::runtime::workspace::WorkspacePackage;

pub(super) fn collect_changed_paths(ctx: &CommandContext) -> XtaskResult<Vec<String>> {
    ctx.workspace().changed_paths()
}

pub(super) fn load_workspace_packages(ctx: &CommandContext) -> XtaskResult<Vec<WorkspacePackage>> {
    ctx.workspace().packages()
}

pub(super) fn detect_changed_packages(
    changed_paths: &[String],
    workspace_packages: &[WorkspacePackage],
) -> Vec<String> {
    workspace_packages
        .iter()
        .filter(|pkg| {
            changed_paths.iter().any(|path| {
                path == &pkg.manifest_dir || path.starts_with(&(pkg.manifest_dir.clone() + "/"))
            })
        })
        .map(|pkg| pkg.name.clone())
        .collect()
}

pub(super) fn looks_like_docs_change(path: &str) -> bool {
    path.starts_with("docs/")
        || path.starts_with("wiki/")
        || path == "AGENTS.md"
        || path == "README.md"
}

pub(super) fn looks_like_workspace_wide_change(path: &str) -> bool {
    matches!(
        path,
        "Cargo.toml" | "Cargo.lock" | ".cargo/config.toml" | VERIFY_PROFILES_FILE
    ) || path == "tools/automation/dev_server.toml"
}

pub(super) fn looks_like_desktop_host_change(path: &str) -> bool {
    path.starts_with("crates/desktop_tauri/")
        || path.starts_with("crates/platform_host/")
        || path.starts_with("crates/platform_host_web/")
        || matches!(path, "Cargo.toml" | "Cargo.lock" | ".cargo/config.toml")
}

pub(super) fn format_package_list(packages: &[String]) -> String {
    if packages.len() <= 4 {
        return packages.join(", ");
    }
    let shown = packages[..4].join(", ");
    format!("{shown} (+{} more)", packages.len() - 4)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::workspace::parse_porcelain_status_path;

    #[test]
    fn desktop_trigger_detection_matches_expected_paths() {
        assert!(looks_like_desktop_host_change(
            "crates/desktop_tauri/src/main.rs"
        ));
        assert!(!looks_like_desktop_host_change(
            "crates/apps/notepad/src/lib.rs"
        ));
    }

    #[test]
    fn changed_package_detection_matches_workspace_path_prefixes() {
        let packages = vec![
            WorkspacePackage {
                name: "site".into(),
                manifest_dir: "crates/site".into(),
            },
            WorkspacePackage {
                name: "xtask".into(),
                manifest_dir: "xtask".into(),
            },
        ];
        let detected = detect_changed_packages(
            &["crates/site/src/main.rs".into(), "README.md".into()],
            &packages,
        );
        assert_eq!(detected, vec!["site"]);
    }

    #[test]
    fn porcelain_parser_handles_rename_records() {
        assert_eq!(
            parse_porcelain_status_path("R  old/path -> new/path"),
            Some("new/path".into())
        );
    }
}
