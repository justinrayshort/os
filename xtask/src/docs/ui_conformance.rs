use super::*;

const FOUNDATIONS_PATH: &str = "crates/site/src/theme_shell/00-foundations.css";
const PRIMITIVES_PATH: &str = "crates/site/src/theme_shell/01-primitives.css";
const SHELL_LAYOUT_PATH: &str = "crates/site/src/theme_shell/02-shell-layout.css";
const RESPONSIVE_PATH: &str = "crates/site/src/theme_shell/03-responsive.css";
const ACCESSIBILITY_PATH: &str = "crates/site/src/theme_shell/04-accessibility-motion.css";
const XP_THEME_PATH: &str = "crates/site/src/theme_shell/10-theme-xp.css";
const LEGACY95_THEME_PATH: &str = "crates/site/src/theme_shell/20-theme-legacy95.css";
const MODERN_THEME_PATH: &str = "crates/site/src/theme_shell/30-theme-modern-adaptive.css";
const SOFT_NEUMORPHIC_THEME_PATH: &str = "crates/site/src/theme_shell/34-theme-soft-neumorphic.css";
const ACTIVE_THEME_SHELL_CSS_FILES: &[&str] = &[
    FOUNDATIONS_PATH,
    PRIMITIVES_PATH,
    SHELL_LAYOUT_PATH,
    RESPONSIVE_PATH,
    ACCESSIBILITY_PATH,
    XP_THEME_PATH,
    LEGACY95_THEME_PATH,
    MODERN_THEME_PATH,
    SOFT_NEUMORPHIC_THEME_PATH,
];
const REQUIRED_SKIN_SCOPES: &[&str] = &[
    ".desktop-shell[data-skin=\"soft-neumorphic\"]",
    ".desktop-shell[data-skin=\"modern-adaptive\"]",
    ".desktop-shell[data-skin=\"classic-xp\"]",
    ".desktop-shell[data-skin=\"classic-95\"]",
];
const SKIN_SCOPED_FILES: &[(&str, &str)] = &[
    (XP_THEME_PATH, ".desktop-shell[data-skin=\"classic-xp\"]"),
    (
        LEGACY95_THEME_PATH,
        ".desktop-shell[data-skin=\"classic-95\"]",
    ),
    (
        MODERN_THEME_PATH,
        ".desktop-shell[data-skin=\"modern-adaptive\"]",
    ),
    (
        SOFT_NEUMORPHIC_THEME_PATH,
        ".desktop-shell[data-skin=\"soft-neumorphic\"]",
    ),
];
const THEME_OVERRIDE_FILES_WITH_LITERAL_HYGIENE: &[&str] = &[];
const TOKEN_ONLY_THEME_FILES: &[&str] = &[
    XP_THEME_PATH,
    LEGACY95_THEME_PATH,
    MODERN_THEME_PATH,
    SOFT_NEUMORPHIC_THEME_PATH,
];
const FORBIDDEN_THEME_SELECTOR_PREFIXES: &[&str] = &[
    ".calc-",
    ".calculator-",
    ".explorer-",
    ".notepad-",
    ".terminal-",
    ".settings-",
    ".app-",
    ".desktop-",
    ".taskbar-",
    ".titlebar-",
    ".tray-",
    ".window-",
    ".tree-",
    ".pane-",
];
const ALLOWED_ROOT_CLASS_SELECTORS: &[&str] =
    &[".site-root", ".desktop-shell", ".canonical-content"];
const SHELL_ICON_COMPONENT_FILES: &[&str] = &[
    "crates/desktop_runtime/src/components.rs",
    "crates/desktop_runtime/src/components/display_properties.rs",
    "crates/desktop_runtime/src/components/menus.rs",
    "crates/desktop_runtime/src/components/taskbar.rs",
    "crates/desktop_runtime/src/components/window.rs",
];
const PRIMITIVE_USAGE_SCAN_DIRS: &[&str] = &["crates/apps", "crates/desktop_runtime/src"];
const FORBIDDEN_LEGACY_PRIMITIVE_TOKENS: &[&str] = &[
    "class=\"app-shell",
    "class=\"app-menubar",
    "class=\"app-toolbar",
    "class=\"app-statusbar",
    "class=\"app-action",
    "class=\"app-field",
    "class=\"app-editor",
    "class=\"app-progress",
    " app-action ",
    " app-field ",
    " app-editor ",
    " app-progress ",
];

pub(crate) fn validate_ui_conformance(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();
    problems.extend(validate_no_data_theme_selectors(root));
    problems.extend(validate_required_skin_scopes(root));
    problems.extend(validate_skin_file_scope_presence(root));
    problems.extend(validate_skin_selector_scoping(root));
    problems.extend(validate_forbidden_theme_selectors(root));
    problems.extend(validate_token_only_theme_files(root));

    for rel_path in THEME_OVERRIDE_FILES_WITH_LITERAL_HYGIENE {
        let path = root.join(rel_path);
        let Ok(text) = fs::read_to_string(&path) else {
            problems.push(Problem::new(
                "ui-conformance",
                *rel_path,
                "failed to read theme overrides CSS for UI conformance checks",
                None,
            ));
            continue;
        };

        for (idx, line) in text.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }
            if trimmed.starts_with("--") {
                continue;
            }

            if has_disallowed_raw_color_literal(trimmed) {
                problems.push(Problem::new(
                    "ui-conformance",
                    *rel_path,
                    "raw color literal outside token definitions (allowed exception: transparent rgba(..., 0) stops)",
                    Some(line_no),
                ));
            }

            if has_disallowed_raw_px_literal(trimmed) {
                problems.push(Problem::new(
                    "ui-conformance",
                    *rel_path,
                    "raw px literal outside token definitions/effect-geometry exceptions",
                    Some(line_no),
                ));
            }
        }
    }

    problems.extend(validate_shell_icon_standardization(root));
    problems.extend(validate_shared_primitive_usage(root));
    problems.extend(validate_system_ui_token_usage(root));
    problems.extend(validate_placeholder_surface_copy(root));
    problems.extend(validate_inline_style_allowlist(root));
    problems.extend(validate_raw_interactive_markup(root));
    problems.extend(validate_app_runtime_layout_contracts(root));

    problems
}

fn validate_forbidden_theme_selectors(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();

    for rel_path in ACTIVE_THEME_SHELL_CSS_FILES {
        let path = root.join(rel_path);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };

        for (idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("/*")
                || trimmed.starts_with('*')
                || trimmed.starts_with('@')
                || trimmed.starts_with('}')
                || !trimmed.contains('{')
            {
                continue;
            }

            let selector_chunk = trimmed
                .split_once('{')
                .map(|(selector, _)| selector.trim())
                .unwrap_or("");

            if selector_chunk.is_empty() || selector_chunk.starts_with("--") {
                continue;
            }

            for selector in selector_chunk.split(',') {
                let selector = selector.trim();
                if selector.is_empty() {
                    continue;
                }
                if ALLOWED_ROOT_CLASS_SELECTORS
                    .iter()
                    .any(|allowed| selector.starts_with(allowed))
                {
                    continue;
                }
                if selector.starts_with("[data-ui-")
                    || selector.starts_with(":root")
                    || selector.starts_with("body")
                {
                    continue;
                }

                if FORBIDDEN_THEME_SELECTOR_PREFIXES
                    .iter()
                    .any(|prefix| selector.contains(prefix))
                {
                    problems.push(Problem::new(
                        "ui-conformance",
                        *rel_path,
                        format!(
                            "forbidden bespoke selector detected in active theme CSS (`{selector}`); style via `data-ui-*` or approved root scopes"
                        ),
                        Some(idx + 1),
                    ));
                }
            }
        }
    }

    problems
}

fn validate_token_only_theme_files(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();

    for rel_path in TOKEN_ONLY_THEME_FILES {
        let path = root.join(rel_path);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };

        for (idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("/*")
                || trimmed.starts_with('*')
                || trimmed.starts_with('@')
                || trimmed.starts_with('}')
                || trimmed.ends_with('{')
            {
                continue;
            }

            if trimmed.starts_with("--") {
                continue;
            }

            if trimmed.contains(':') {
                problems.push(Problem::new(
                    "ui-conformance",
                    *rel_path,
                    "theme token file contains a non-token declaration; skin files must only remap `--sys-*` tokens",
                    Some(idx + 1),
                ));
            }
        }
    }

    problems
}

fn validate_no_data_theme_selectors(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();
    for rel_path in ACTIVE_THEME_SHELL_CSS_FILES {
        let path = root.join(rel_path);
        let Ok(text) = fs::read_to_string(&path) else {
            problems.push(Problem::new(
                "ui-conformance",
                *rel_path,
                "failed to read active split CSS source while scanning for deprecated data-theme selectors",
                None,
            ));
            continue;
        };
        for (idx, line) in text.lines().enumerate() {
            if line.contains("data-theme") {
                problems.push(Problem::new(
                    "ui-conformance",
                    *rel_path,
                    "deprecated `data-theme` selector detected; use `data-skin` scopes",
                    Some(idx + 1),
                ));
            }
        }
    }
    problems
}

fn validate_required_skin_scopes(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut joined = String::new();

    for rel_path in ACTIVE_THEME_SHELL_CSS_FILES {
        let path = root.join(rel_path);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        joined.push_str(&text);
        joined.push('\n');
    }

    for scope in REQUIRED_SKIN_SCOPES {
        if !joined.contains(scope) {
            problems.push(Problem::new(
                "ui-conformance",
                "crates/site/src/theme_shell",
                format!("missing required skin scope `{scope}` in active split CSS"),
                None,
            ));
        }
    }

    problems
}

fn validate_skin_file_scope_presence(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();

    for (rel_path, scope) in SKIN_SCOPED_FILES {
        let path = root.join(rel_path);
        let Ok(text) = fs::read_to_string(&path) else {
            problems.push(Problem::new(
                "ui-conformance",
                *rel_path,
                "failed to read skin CSS file for scope validation",
                None,
            ));
            continue;
        };

        if !text.contains(scope) {
            problems.push(Problem::new(
                "ui-conformance",
                *rel_path,
                format!("skin CSS file is missing required scope prefix `{scope}`"),
                None,
            ));
        }
    }

    problems
}

fn validate_skin_selector_scoping(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();

    for (rel_path, scope_prefix) in SKIN_SCOPED_FILES {
        let path = root.join(rel_path);
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("/*")
                || trimmed.starts_with('*')
                || trimmed.starts_with("*/")
            {
                continue;
            }

            if trimmed.starts_with('@') || trimmed.starts_with('}') || !trimmed.contains('{') {
                continue;
            }

            let selector_chunk = trimmed
                .split_once('{')
                .map(|(selector, _)| selector.trim())
                .unwrap_or("");

            if selector_chunk.is_empty()
                || selector_chunk.starts_with(':')
                || selector_chunk.starts_with("--")
            {
                continue;
            }

            if selector_chunk == "from"
                || selector_chunk == "to"
                || selector_chunk.ends_with('%')
                || (selector_chunk.contains(':') && !selector_chunk.contains(','))
            {
                continue;
            }

            for selector in selector_chunk.split(',') {
                let selector = selector.trim();
                if selector.is_empty()
                    || selector.starts_with(':')
                    || selector == "from"
                    || selector == "to"
                    || selector.ends_with('%')
                {
                    continue;
                }
                if !selector.starts_with(scope_prefix) {
                    problems.push(Problem::new(
                        "ui-conformance",
                        *rel_path,
                        format!("unscoped selector in skin file; expected prefix `{scope_prefix}`"),
                        Some(idx + 1),
                    ));
                }
            }
        }
    }

    problems
}

fn validate_shell_icon_standardization(root: &Path) -> Vec<Problem> {
    /* omitted for brevity in patch? */
    let mut problems = Vec::new();
    for rel_path in SHELL_ICON_COMPONENT_FILES {
        let path = root.join(rel_path);
        let Ok(text) = fs::read_to_string(&path) else {
            problems.push(Problem::new(
                "ui-conformance",
                *rel_path,
                "failed to read shell component file for icon standardization checks",
                None,
            ));
            continue;
        };
        for (idx, line) in text.lines().enumerate() {
            let line_no = idx + 1;
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            if trimmed.contains("<svg")
                || trimmed.contains("inner_html=")
                || trimmed.contains("path d=\"")
            {
                problems.push(Problem::new(
                    "ui-conformance",
                    *rel_path,
                    "inline icon markup detected in shell component; use `FluentIcon` + `IconName`",
                    Some(line_no),
                ));
            }
            if contains_legacy_shell_icon_text_glyph(trimmed) {
                problems.push(Problem::new("ui-conformance", *rel_path, "legacy text glyph icon marker detected in shell component; use semantic `IconName`/`FluentIcon`", Some(line_no)));
            }
        }
    }
    problems
}

fn validate_shared_primitive_usage(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();
    for rel_dir in PRIMITIVE_USAGE_SCAN_DIRS {
        let dir = root.join(rel_dir);
        if !dir.exists() {
            continue;
        }
        let mut files = match collect_files_with_suffix(&dir, ".rs") {
            Ok(files) => files,
            Err(err) => {
                problems.push(Problem::new(
                    "ui-conformance",
                    *rel_dir,
                    format!("failed to scan Rust files for primitive usage checks: {err}"),
                    None,
                ));
                continue;
            }
        };
        files.sort();
        for path in files {
            let rel_path = rel_posix(root, &path);
            if rel_path.starts_with("crates/system_ui/") {
                continue;
            }
            let Ok(text) = fs::read_to_string(&path) else {
                problems.push(Problem::new(
                    "ui-conformance",
                    rel_path,
                    "failed to read Rust file for primitive usage validation",
                    None,
                ));
                continue;
            };
            for (idx, line) in text.lines().enumerate() {
                let line_no = idx + 1;
                let trimmed = line.trim();
                if trimmed.starts_with("//") {
                    continue;
                }
                for token in FORBIDDEN_LEGACY_PRIMITIVE_TOKENS {
                    if trimmed.contains(token) {
                        problems.push(Problem::new("ui-conformance", rel_path.clone(), format!("legacy primitive class usage detected (`{token}`); consume shared `system_ui` primitives or `ui-*`/`data-ui-*` roots"), Some(line_no)));
                    }
                }
                if trimmed.contains("desktop_runtime::icons")
                    || trimmed.contains("crate::icons")
                    || trimmed.contains("use crate::icons")
                {
                    problems.push(Problem::new(
                        "ui-conformance",
                        rel_path.clone(),
                        "old icon import path detected; import shared icons from `system_ui`",
                        Some(line_no),
                    ));
                }
            }
        }
    }
    problems
}

fn validate_system_ui_token_usage(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();
    let dir = root.join("crates/system_ui/src");
    let mut files = match collect_files_with_suffix(&dir, ".rs") {
        Ok(files) => files,
        Err(err) => {
            problems.push(Problem::new(
                "ui-conformance",
                "crates/system_ui/src",
                format!("failed to scan shared primitive sources: {err}"),
                None,
            ));
            return problems;
        }
    };
    files.sort();
    for path in files {
        let rel_path = rel_posix(root, &path);
        let Ok(text) = fs::read_to_string(&path) else {
            problems.push(Problem::new(
                "ui-conformance",
                rel_path,
                "failed to read shared primitive source for token validation",
                None,
            ));
            continue;
        };
        for (idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            if trimmed.contains("--neuro-")
                || trimmed.contains("--fluent-")
                || trimmed.contains("--ui-")
            {
                problems.push(Problem::new(
                    "ui-conformance",
                    rel_path.clone(),
                    "shared primitive sources must not depend on skin-specific token families",
                    Some(idx + 1),
                ));
            }
        }
    }
    problems
}

fn validate_placeholder_surface_copy(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();
    let scan_files = [
        "crates/desktop_runtime/src/apps/placeholders.rs",
        "crates/apps/settings/src/lib.rs",
        "crates/apps/explorer/src/lib.rs",
        "crates/apps/notepad/src/lib.rs",
        "crates/apps/calculator/src/lib.rs",
        "crates/apps/terminal/src/lib.rs",
    ];
    let forbidden_markers = [
        "coming soon",
        "negotiating connection...",
        "placeholder ready",
        "placeholder save slot",
        "(Placeholder)",
    ];
    for rel_path in scan_files {
        let path = root.join(rel_path);
        let Ok(text) = fs::read_to_string(&path) else {
            problems.push(Problem::new(
                "ui-conformance",
                rel_path,
                "failed to read app surface source for placeholder copy validation",
                None,
            ));
            continue;
        };
        for (idx, line) in text.lines().enumerate() {
            let lowered = line.to_ascii_lowercase();
            if forbidden_markers
                .iter()
                .any(|marker| lowered.contains(marker))
            {
                problems.push(Problem::new("ui-conformance", rel_path, "placeholder-grade copy detected in built-in app surface; replace with truthful limited-scope copy or real state", Some(idx + 1)));
            }
        }
    }
    problems
}

fn validate_inline_style_allowlist(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();
    for rel_dir in PRIMITIVE_USAGE_SCAN_DIRS {
        let dir = root.join(rel_dir);
        if !dir.exists() {
            continue;
        }
        let mut files = match collect_files_with_suffix(&dir, ".rs") {
            Ok(files) => files,
            Err(_) => continue,
        };
        files.sort();
        for path in files {
            let rel_path = rel_posix(root, &path);
            let Ok(text) = fs::read_to_string(&path) else {
                continue;
            };
            for (idx, line) in text.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with("//") || !trimmed.contains("style=") {
                    continue;
                }
                if is_allowed_inline_style(&rel_path, trimmed) {
                    continue;
                }
                problems.push(Problem::new(
                    "ui-conformance",
                    rel_path.clone(),
                    "inline style detected outside the runtime geometry/media-position allowlist",
                    Some(idx + 1),
                ));
            }
        }
    }
    problems
}

fn is_allowed_inline_style(rel_path: &str, line: &str) -> bool {
    let _ = line;
    matches!(
        rel_path,
        "crates/desktop_runtime/src/components.rs"
            | "crates/desktop_runtime/src/components/window.rs"
            | "crates/desktop_runtime/src/components/menus.rs"
            | "crates/system_ui/src/primitives/overlays.rs"
            | "crates/system_ui/src/primitives/shell.rs"
    )
}

fn validate_raw_interactive_markup(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();
    let forbidden_primitive_kinds = [
        "pane",
        "pane-header",
        "split-layout",
        "list-surface",
        "terminal-surface",
        "terminal-transcript",
        "terminal-prompt",
        "completion-list",
        "completion-item",
        "empty-state",
        "panel",
        "tree",
        "tree-item",
        "card",
        "modal",
        "field-group",
        "statusbar-item",
        "checkbox",
    ];
    for (scan_dir, message) in [("crates/apps","raw interactive element detected in app crate; use approved `system_ui` primitives"),("crates/desktop_runtime/src/components","raw interactive element detected in runtime shell surface; compose through shared `system_ui` primitives")] { let dir = root.join(scan_dir); let mut files = match collect_files_with_suffix(&dir, ".rs") { Ok(files) => files, Err(_) => continue }; files.sort(); for path in files { let rel_path = rel_posix(root, &path); let Ok(text) = fs::read_to_string(&path) else { continue; }; for (idx, line) in text.lines().enumerate() { let trimmed = line.trim(); if trimmed.starts_with("//") { continue; } let forbidden = trimmed.contains("<button") || trimmed.contains("<input") || trimmed.contains("<textarea") || trimmed.contains("<table") || trimmed.contains("<select"); if forbidden { problems.push(Problem::new("ui-conformance", rel_path.clone(), message, Some(idx + 1))); } if let Some(kind) = forbidden_primitive_kinds.iter().find(|kind| trimmed.contains(&format!("data-ui-kind=\"{kind}\""))) { problems.push(Problem::new("ui-conformance", rel_path.clone(), format!("direct `data-ui-kind=\"{kind}\"` composition detected outside `system_ui`; use the approved primitive component instead"), Some(idx + 1))); } } } }
    problems
}

fn validate_app_runtime_layout_contracts(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();
    let Ok(entries) = collect_ui_inventory(root) else {
        return problems;
    };
    for entry in entries {
        if entry.classification != "layout_only" {
            continue;
        }
        if !matches!(
            entry.owner_layer.as_str(),
            "app_crate" | "desktop_runtime_shell"
        ) {
            continue;
        }
        problems.push(Problem::new("ui-conformance", entry.file, format!("layout-only class contract `{}` detected in app/runtime surface; replace it with shared primitive props, `data-ui-*` semantics, or remove the unused hook", entry.selector_or_token), Some(entry.line)));
    }
    problems
}

pub(crate) fn write_ui_inventory(root: &Path, output: &Path) -> XtaskResult<()> {
    let entries = collect_ui_inventory(root)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            XtaskError::io(format!("failed to create {}: {err}", parent.display()))
                .with_operation("write UI inventory")
                .with_path(parent)
        })?;
    }
    let json = serde_json::to_string_pretty(&entries)
        .map_err(|err| XtaskError::io(format!("failed to serialize UI inventory: {err}")))?;
    fs::write(output, json).map_err(|err| {
        XtaskError::io(format!("failed to write {}: {err}", output.display()))
            .with_operation("write UI inventory")
            .with_path(output)
    })?;
    println!("UI inventory entries: {}", entries.len());
    println!("Wrote {}", output.display());
    Ok(())
}

fn collect_ui_inventory(root: &Path) -> XtaskResult<Vec<UiInventoryEntry>> {
    let mut entries = Vec::new();
    let consumed_css_classes = collect_consumed_css_classes(root)?;
    for rel_dir in [
        "crates/apps",
        "crates/desktop_runtime/src",
        "crates/system_ui/src",
    ] {
        let dir = root.join(rel_dir);
        let mut files = collect_files_with_suffix(&dir, ".rs")?;
        files.sort();
        for path in files {
            let rel_path = rel_posix(root, &path);
            let text = fs::read_to_string(&path).map_err(|err| {
                XtaskError::io(format!("failed to read {}: {err}", path.display()))
                    .with_operation("collect UI inventory")
                    .with_path(&path)
            })?;
            let owner_layer = rust_owner_layer(&rel_path).to_string();
            for (idx, line) in text.lines().enumerate() {
                if let Some(token) = extract_attr_literal(line, "class=\"") {
                    entries.push(UiInventoryEntry { entrypoint_type: "rust_class".to_string(), owner_layer: owner_layer.clone(), selector_or_token: token.clone(), file: rel_path.clone(), line: idx + 1, classification: classify_rust_contract(&token, &consumed_css_classes).to_string(), recommended_replacement: "Replace bespoke classes with `data-ui-*` primitives or layout-only hooks.".to_string(), });
                }
                if let Some(token) = extract_attr_literal(line, "layout_class=\"") {
                    entries.push(UiInventoryEntry { entrypoint_type: "rust_layout_class".to_string(), owner_layer: owner_layer.clone(), selector_or_token: token.clone(), file: rel_path.clone(), line: idx + 1, classification: classify_rust_contract(&token, &consumed_css_classes).to_string(), recommended_replacement: "Keep only layout/test hooks; do not consume layout classes from theme CSS.".to_string(), });
                }
                if line.contains("style=") {
                    entries.push(UiInventoryEntry {
                        entrypoint_type: "rust_inline_style".to_string(),
                        owner_layer: owner_layer.clone(),
                        selector_or_token: "style=".to_string(),
                        file: rel_path.clone(),
                        line: idx + 1,
                        classification: if is_allowed_inline_style(&rel_path, line) {
                            "exception".to_string()
                        } else {
                            "legacy_visual_contract".to_string()
                        },
                        recommended_replacement:
                            "Restrict inline styles to runtime geometry/media positioning."
                                .to_string(),
                    });
                }
            }
        }
    }
    let css_dir = root.join("crates/site/src/theme_shell");
    let mut css_files = collect_files_with_suffix(&css_dir, ".css")?;
    css_files.sort();
    for path in css_files {
        let rel_path = rel_posix(root, &path);
        let text = fs::read_to_string(&path).map_err(|err| {
            XtaskError::io(format!("failed to read {}: {err}", path.display()))
                .with_operation("collect UI inventory")
                .with_path(&path)
        })?;
        for (idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("--") {
                entries.push(UiInventoryEntry {
                    entrypoint_type: "token_definition".to_string(),
                    owner_layer: "theme_shell".to_string(),
                    selector_or_token: trimmed.split(':').next().unwrap_or(trimmed).to_string(),
                    file: rel_path.clone(),
                    line: idx + 1,
                    classification: "approved".to_string(),
                    recommended_replacement: "Keep all design-token definitions under `--sys-*`."
                        .to_string(),
                });
            } else if trimmed.contains('{') && !trimmed.starts_with('@') {
                let selector = trimmed
                    .split_once('{')
                    .map(|(selector, _)| selector.trim())
                    .unwrap_or(trimmed)
                    .to_string();
                entries.push(UiInventoryEntry { entrypoint_type: "css_selector".to_string(), owner_layer: "theme_shell".to_string(), selector_or_token: selector.clone(), file: rel_path.clone(), line: idx + 1, classification: classify_css_selector(&selector).to_string(), recommended_replacement: "Active theme CSS should target `.desktop-shell` scopes or `data-ui-*` selectors only.".to_string(), });
            } else if has_disallowed_raw_color_literal(trimmed) {
                entries.push(UiInventoryEntry {
                    entrypoint_type: "css_literal".to_string(),
                    owner_layer: "theme_shell".to_string(),
                    selector_or_token: trimmed.to_string(),
                    file: rel_path.clone(),
                    line: idx + 1,
                    classification: "hard_coded_literal".to_string(),
                    recommended_replacement: "Move visual literals into `--sys-*` tokens."
                        .to_string(),
                });
            }
        }
    }
    Ok(entries)
}

pub(crate) fn extract_attr_literal(line: &str, needle: &str) -> Option<String> {
    let search = needle.as_bytes();
    let bytes = line.as_bytes();
    let mut idx = 0usize;

    while idx + search.len() <= bytes.len() {
        if &bytes[idx..idx + search.len()] == search {
            let boundary_ok = idx == 0
                || !(bytes[idx - 1].is_ascii_alphanumeric()
                    || bytes[idx - 1] == b'_'
                    || bytes[idx - 1] == b'-');
            if boundary_ok {
                let start = idx + needle.len();
                let end = line[start..].find('"')?;
                return Some(line[start..start + end].to_string());
            }
        }
        idx += 1;
    }

    None
}

fn collect_consumed_css_classes(root: &Path) -> XtaskResult<HashSet<String>> {
    let mut classes = HashSet::new();
    let css_dir = root.join("crates/site/src/theme_shell");
    let mut css_files = collect_files_with_suffix(&css_dir, ".css")?;
    css_files.sort();
    for path in css_files {
        let text = fs::read_to_string(&path).map_err(|err| {
            XtaskError::io(format!("failed to read {}: {err}", path.display()))
                .with_operation("collect consumed CSS classes")
                .with_path(&path)
        })?;
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("/*")
                || trimmed.starts_with('*')
                || trimmed.starts_with('@')
                || !trimmed.contains('{')
            {
                continue;
            }
            let selector_chunk = trimmed
                .split_once('{')
                .map(|(selector, _)| selector.trim())
                .unwrap_or("");
            for selector in selector_chunk.split(',') {
                let selector = selector.trim();
                let bytes = selector.as_bytes();
                let mut idx = 0;
                while idx < bytes.len() {
                    if bytes[idx] == b'.' {
                        let start = idx + 1;
                        let mut end = start;
                        while end < bytes.len() {
                            let ch = bytes[end] as char;
                            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                                end += 1;
                            } else {
                                break;
                            }
                        }
                        if end > start {
                            classes.insert(selector[start..end].to_string());
                        }
                        idx = end;
                    } else {
                        idx += 1;
                    }
                }
            }
        }
    }
    Ok(classes)
}

fn rust_owner_layer(rel_path: &str) -> &'static str {
    if rel_path.starts_with("crates/system_ui/") {
        "system_ui"
    } else if rel_path.starts_with("crates/desktop_runtime/src/components") {
        "desktop_runtime_shell"
    } else {
        "app_crate"
    }
}

fn classify_rust_contract(token: &str, consumed_css_classes: &HashSet<String>) -> &'static str {
    let mut saw_non_root_class = false;
    for class_name in token.split_whitespace() {
        if ALLOWED_ROOT_CLASS_SELECTORS
            .iter()
            .any(|allowed| class_name == allowed.trim_start_matches('.'))
        {
            continue;
        }
        saw_non_root_class = true;
        if FORBIDDEN_THEME_SELECTOR_PREFIXES.iter().any(|prefix| {
            class_name.contains(prefix.trim_start_matches('.'))
                && consumed_css_classes.contains(class_name)
        }) {
            return "legacy_visual_contract";
        }
    }
    if saw_non_root_class {
        "layout_only"
    } else {
        "approved"
    }
}

fn classify_css_selector(selector: &str) -> &'static str {
    if ALLOWED_ROOT_CLASS_SELECTORS
        .iter()
        .any(|allowed| selector.starts_with(allowed))
        || selector.starts_with(".desktop-shell[")
        || selector.starts_with("[data-ui-")
        || selector.starts_with(":root")
        || selector.starts_with("body")
    {
        if selector.contains(".desktop-shell[data-skin=") {
            "skin_override"
        } else {
            "approved"
        }
    } else if FORBIDDEN_THEME_SELECTOR_PREFIXES
        .iter()
        .any(|prefix| selector.contains(prefix))
    {
        "legacy_visual_contract"
    } else if selector.contains(".desktop-shell[data-skin=") {
        "skin_override"
    } else {
        "approved"
    }
}

fn contains_legacy_shell_icon_text_glyph(line: &str) -> bool {
    const LEGACY_MARKERS: &[&str] = &["\"DIR\"", "\"TXT\"", "\"56K\"", "'DIR'", "'TXT'", "'56K'"];
    LEGACY_MARKERS.iter().any(|marker| line.contains(marker))
}

fn has_disallowed_raw_color_literal(line: &str) -> bool {
    let has_rgba = line.contains("rgba(");
    let has_hex = contains_hex_color_literal(line);
    if !has_rgba && !has_hex {
        return false;
    }
    if has_hex {
        return true;
    }
    let mut rest = line;
    while let Some(start) = rest.find("rgba(") {
        let after = &rest[start + 5..];
        let Some(end) = after.find(')') else {
            return true;
        };
        let args = &after[..end];
        if !is_transparent_rgba_stop(args) {
            return true;
        }
        rest = &after[end + 1..];
    }
    false
}

fn is_transparent_rgba_stop(args: &str) -> bool {
    let parts: Vec<_> = args.split(',').map(|p| p.trim()).collect();
    parts.len() == 4 && parts[3] == "0"
}

fn contains_hex_color_literal(line: &str) -> bool {
    let bytes = line.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] != b'#' {
            continue;
        }
        let mut count = 0usize;
        let mut j = i + 1;
        while j < bytes.len() && bytes[j].is_ascii_hexdigit() && count < 8 {
            count += 1;
            j += 1;
        }
        if matches!(count, 3 | 4 | 6 | 8) {
            return true;
        }
    }
    false
}

fn has_disallowed_raw_px_literal(line: &str) -> bool {
    if !line.contains("px") || !contains_px_number(line) {
        return false;
    }
    if is_px_effect_geometry_exception(line) {
        return false;
    }
    true
}

fn contains_px_number(line: &str) -> bool {
    let bytes = line.as_bytes();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] != b'p' || bytes[i + 1] != b'x' || i == 0 {
            continue;
        }
        let mut j = i;
        let mut saw_digit = false;
        while j > 0 {
            let c = bytes[j - 1];
            if c.is_ascii_digit() {
                saw_digit = true;
                j -= 1;
                continue;
            }
            break;
        }
        if saw_digit {
            return true;
        }
    }
    false
}

fn is_px_effect_geometry_exception(line: &str) -> bool {
    let effect_keywords = [
        "radial-gradient(",
        "linear-gradient(",
        "text-shadow:",
        "box-shadow:",
        "outline:",
        "outline-offset:",
        "transform:",
        "@supports (backdrop-filter:",
        "border:",
        "border-top:",
        "border-bottom:",
    ];
    if effect_keywords.iter().any(|kw| line.contains(kw)) {
        return true;
    }
    line.contains("inset 0 ")
        || line.starts_with("0 ")
        || line.contains("transparent 60%")
        || line.contains("transparent 62%")
        || line.contains("transparent 64%")
        || line.contains("transparent 70%")
        || line.contains("transparent 72%")
        || line.contains("transparent 74%")
        || line.contains("transparent 58%")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_attr_literal_does_not_match_substrings_inside_other_attribute_names() {
        assert_eq!(
            extract_attr_literal("layout_class=\"foo\"", "class=\""),
            None
        );
        assert_eq!(
            extract_attr_literal(" class=\"foo\"", "class=\""),
            Some("foo".into())
        );
    }
}
