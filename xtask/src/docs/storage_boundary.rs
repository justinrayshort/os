use super::*;

const TYPED_PERSISTENCE_BOUNDARY_DIRS: &[&str] =
    &["crates/apps", "crates/desktop_runtime", "crates/site"];
const PLATFORM_HOST_WEB_IMPORT_ALLOWLIST: &[&str] = &["crates/site/src/web_app.rs"];

pub(crate) fn validate_typed_persistence_boundary(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();

    for rel_dir in TYPED_PERSISTENCE_BOUNDARY_DIRS {
        let dir = root.join(rel_dir);
        if !dir.exists() {
            continue;
        }

        let mut files = match collect_files_with_suffix(&dir, ".rs") {
            Ok(files) => files,
            Err(err) => {
                problems.push(Problem::new(
                    "storage-boundary",
                    *rel_dir,
                    format!("failed to scan Rust files: {err}"),
                    None,
                ));
                continue;
            }
        };
        files.sort();

        for path in files {
            let rel_path = rel_posix(root, &path);
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(err) => {
                    problems.push(Problem::new(
                        "storage-boundary",
                        rel_path,
                        format!("failed to read file: {err}"),
                        None,
                    ));
                    continue;
                }
            };
            let imports_low_level_load = imports_legacy_low_level_app_state_load(&text);
            let allows_platform_host_web =
                PLATFORM_HOST_WEB_IMPORT_ALLOWLIST.contains(&rel_path.as_str());

            for (idx, line) in text.lines().enumerate() {
                if uses_forbidden_envelope_load_call(line, imports_low_level_load) {
                    problems.push(Problem::new(
                        "storage-boundary",
                        rel_path.clone(),
                        "direct `load_app_state_envelope(...)` usage is not allowed in app/runtime crates; use typed load helpers",
                        Some(idx + 1),
                    ));
                }
                if !allows_platform_host_web && uses_forbidden_platform_host_web_import(line) {
                    problems.push(Problem::new(
                        "storage-boundary",
                        rel_path.clone(),
                        "direct `platform_host_web` imports are not allowed here; route host access through `AppServices`, `DesktopHostContext`, or the entry-layer host bundle assembly in `site`",
                        Some(idx + 1),
                    ));
                }
            }
        }
    }

    problems
}

fn imports_legacy_low_level_app_state_load(text: &str) -> bool {
    text.lines().any(|line| {
        let compact = line
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>();
        compact.contains("useplatform_storage::load_app_state_envelope")
            || (compact.contains("useplatform_storage::{")
                && compact.contains("load_app_state_envelope"))
    })
}

fn uses_forbidden_envelope_load_call(line: &str, imports_low_level_load: bool) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return false;
    }
    let compact = line
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();

    if compact.contains("platform_storage::load_app_state_envelope(") {
        return true;
    }

    if imports_low_level_load
        && compact.contains("load_app_state_envelope(")
        && !compact.contains(".load_app_state_envelope(")
        && !compact.contains("fnload_app_state_envelope(")
    {
        return true;
    }

    false
}

fn uses_forbidden_platform_host_web_import(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return false;
    }

    let compact = line
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>();

    compact.starts_with("useplatform_host_web") || compact.contains("externcrateplatform_host_web")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_boundary_detects_legacy_low_level_import_patterns() {
        assert!(imports_legacy_low_level_app_state_load(
            "use platform_storage::load_app_state_envelope;"
        ));
        assert!(imports_legacy_low_level_app_state_load(
            "use platform_storage::{foo, load_app_state_envelope, bar};"
        ));
    }

    #[test]
    fn storage_boundary_allows_comments_and_flags_calls() {
        assert!(!uses_forbidden_envelope_load_call(
            "// load_app_state_envelope()",
            true
        ));
        assert!(uses_forbidden_envelope_load_call(
            "let _ = load_app_state_envelope(\"app\");",
            true
        ));
    }

    #[test]
    fn storage_boundary_flags_platform_host_web_imports_outside_allowlist() {
        assert!(uses_forbidden_platform_host_web_import(
            "use platform_host_web::foo::Bar;"
        ));
        assert!(!uses_forbidden_platform_host_web_import(
            "// use platform_host_web::foo::Bar;"
        ));
    }
}
