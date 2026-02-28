use super::*;

pub(crate) fn validate_app_contracts(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();
    let apps_root = root.join("crates/apps");
    if !apps_root.exists() {
        return problems;
    }

    let mut app_dirs = match fs::read_dir(&apps_root) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>(),
        Err(err) => {
            problems.push(Problem::new(
                "app-contract",
                "crates/apps",
                format!("failed to list app directories: {err}"),
                None,
            ));
            return problems;
        }
    };
    app_dirs.sort();

    for app_dir in app_dirs {
        let app_name = app_dir
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("unknown");
        let manifest_path = app_dir.join("app.manifest.toml");
        if !manifest_path.exists() {
            problems.push(Problem::new(
                "app-contract",
                rel_posix(root, &manifest_path),
                "missing required app.manifest.toml",
                None,
            ));
            continue;
        }

        let raw = match fs::read_to_string(&manifest_path) {
            Ok(raw) => raw,
            Err(err) => {
                problems.push(Problem::new(
                    "app-contract",
                    rel_posix(root, &manifest_path),
                    format!("failed to read manifest: {err}"),
                    None,
                ));
                continue;
            }
        };

        let required = [
            "schema_version",
            "app_id",
            "display_name",
            "version",
            "runtime_contract_version",
            "requested_capabilities",
            "single_instance",
            "suspend_policy",
            "show_in_launcher",
            "show_on_desktop",
            "window_defaults",
        ];
        for key in required {
            if !raw.contains(key) {
                problems.push(Problem::new(
                    "app-contract",
                    rel_posix(root, &manifest_path),
                    format!("manifest missing required key `{key}`"),
                    None,
                ));
            }
        }

        if let Some(app_id_line) = raw
            .lines()
            .find(|line| line.trim_start().starts_with("app_id"))
        {
            if let Some((_, value)) = app_id_line.split_once('=') {
                let id = strip_quotes(value.trim());
                if !is_valid_namespaced_app_id(id) {
                    problems.push(Problem::new(
                        "app-contract",
                        rel_posix(root, &manifest_path),
                        format!("invalid namespaced app_id `{id}`"),
                        None,
                    ));
                }
            }
        }

        let src = app_dir.join("src/lib.rs");
        if src.exists() {
            if let Ok(text) = fs::read_to_string(&src) {
                if text.contains("AppHost") {
                    problems.push(Problem::new(
                        "app-contract",
                        rel_posix(root, &src),
                        "AppHost usage is disallowed; use AppServices injection",
                        None,
                    ));
                }
            }
        } else {
            problems.push(Problem::new(
                "app-contract",
                rel_posix(root, &app_dir),
                format!("app `{app_name}` missing src/lib.rs"),
                None,
            ));
        }
    }

    problems
}

fn is_valid_namespaced_app_id(id: &str) -> bool {
    if id.is_empty() || !id.contains('.') {
        return false;
    }
    id.split('.').all(|seg| {
        !seg.is_empty()
            && seg
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
            && seg
                .chars()
                .next()
                .map(|ch| ch.is_ascii_lowercase())
                .unwrap_or(false)
    })
}
