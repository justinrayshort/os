use super::*;

pub(crate) fn validate_wiki_submodule(root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();
    let gitmodules_path = root.join(".gitmodules");
    if !gitmodules_path.exists() {
        problems.push(Problem::new(
            "wiki",
            ".gitmodules",
            "missing .gitmodules; expected `wiki` submodule",
            None,
        ));
        return problems;
    }

    let gitmodules_text = match fs::read_to_string(&gitmodules_path) {
        Ok(text) => text,
        Err(err) => {
            problems.push(Problem::new(
                "wiki",
                ".gitmodules",
                format!("failed to read .gitmodules: {err}"),
                None,
            ));
            return problems;
        }
    };

    let wiki_section = parse_gitmodules_section(&gitmodules_text, r#"submodule "wiki""#);
    match wiki_section {
        None => problems.push(Problem::new(
            "wiki",
            ".gitmodules",
            "missing `[submodule \"wiki\"]` section",
            None,
        )),
        Some(values) => {
            let path_value = values.get("path").cloned().unwrap_or_default();
            let url_value = values.get("url").cloned().unwrap_or_default();
            if path_value != WIKI_SUBMODULE_PATH {
                let found = if path_value.is_empty() {
                    "<missing>"
                } else {
                    &path_value
                };
                problems.push(Problem::new(
                    "wiki",
                    ".gitmodules",
                    format!(
                        "`wiki` submodule path must be `{}` (found `{found}`)",
                        WIKI_SUBMODULE_PATH
                    ),
                    None,
                ));
            }
            if url_value != WIKI_SUBMODULE_URL {
                let found = if url_value.is_empty() {
                    "<missing>"
                } else {
                    &url_value
                };
                problems.push(Problem::new(
                    "wiki",
                    ".gitmodules",
                    format!(
                        "`wiki` submodule url must be `{}` (found `{found}`)",
                        WIKI_SUBMODULE_URL
                    ),
                    None,
                ));
            }
        }
    }

    let wiki_root = root.join(WIKI_SUBMODULE_PATH);
    if !wiki_root.exists() {
        problems.push(Problem::new(
            "wiki",
            WIKI_SUBMODULE_PATH,
            "wiki submodule is not initialized; run `git submodule update --init --recursive`",
            None,
        ));
        return problems;
    }

    if !wiki_root.join(".git").exists() {
        problems.push(Problem::new(
            "wiki",
            WIKI_SUBMODULE_PATH,
            "expected `wiki/` to be a git submodule working tree (`wiki/.git` missing)",
            None,
        ));
    }

    for page in REQUIRED_WIKI_PAGES {
        let page_path = wiki_root.join(page);
        if !page_path.exists() {
            problems.push(Problem::new(
                "wiki",
                format!("{WIKI_SUBMODULE_PATH}/{page}"),
                "required wiki page is missing",
                None,
            ));
        }
    }

    let home_path = wiki_root.join("Home.md");
    if let Ok(home_text) = fs::read_to_string(&home_path) {
        if !home_text.contains("Diataxis") {
            problems.push(Problem::new(
                "wiki",
                format!("{WIKI_SUBMODULE_PATH}/Home.md"),
                "Home page should describe the Diataxis organization of the wiki",
                None,
            ));
        }
        if !home_text.to_lowercase().contains("rustdoc") {
            problems.push(Problem::new(
                "wiki",
                format!("{WIKI_SUBMODULE_PATH}/Home.md"),
                "Home page should point readers to rustdoc for API/reference material",
                None,
            ));
        }
    }

    let os_wiki_path = wiki_root.join("OS-Wiki.md");
    if let Ok(os_wiki_text) = fs::read_to_string(&os_wiki_path) {
        if !os_wiki_text.contains("Home") && !os_wiki_text.contains("OS Wiki") {
            problems.push(Problem::new(
                "wiki",
                format!("{WIKI_SUBMODULE_PATH}/OS-Wiki.md"),
                "OS-Wiki alias page should point readers to Home/primary wiki navigation",
                None,
            ));
        }
    }

    let sidebar_path = wiki_root.join("_Sidebar.md");
    if let Ok(sidebar_text) = fs::read_to_string(&sidebar_path) {
        let sidebar_lower = sidebar_text.to_lowercase();
        for expected in ["Tutorials", "How-To", "Explanations", "API Reference"] {
            if !sidebar_lower.contains(&expected.to_lowercase()) {
                problems.push(Problem::new(
                    "wiki",
                    format!("{WIKI_SUBMODULE_PATH}/_Sidebar.md"),
                    format!("sidebar should include `{expected}` navigation entry"),
                    None,
                ));
            }
        }
    }

    problems.extend(validate_wiki_instructional_templates(&wiki_root));

    problems
}

#[derive(Clone, Debug)]
struct MarkdownHeadingRecord {
    level: usize,
    text: String,
    line: usize,
}

fn validate_wiki_instructional_templates(wiki_root: &Path) -> Vec<Problem> {
    let mut problems = Vec::new();
    let mut files = match collect_files_with_suffix(wiki_root, ".md") {
        Ok(files) => files,
        Err(err) => {
            problems.push(Problem::new(
                "wiki",
                WIKI_SUBMODULE_PATH,
                format!("failed to scan wiki pages for instructional template validation: {err}"),
                None,
            ));
            return problems;
        }
    };
    files.sort();

    for path in files {
        let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if !is_wiki_instructional_page(file_name) {
            continue;
        }

        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) => {
                problems.push(Problem::new(
                    "wiki",
                    rel_posix(wiki_root.parent().unwrap_or(wiki_root), &path),
                    format!("failed to read wiki instructional page: {err}"),
                    None,
                ));
                continue;
            }
        };

        problems.extend(validate_wiki_instructional_page_template(file_name, &text));
    }

    problems
}

fn is_wiki_instructional_page(file_name: &str) -> bool {
    (file_name.starts_with("Tutorial-") || file_name.starts_with("How-to-"))
        && file_name.ends_with(".md")
}

pub(crate) fn validate_wiki_instructional_page_template(
    file_name: &str,
    text: &str,
) -> Vec<Problem> {
    let mut problems = Vec::new();
    let headings = extract_markdown_heading_records(text);
    let path = format!("{WIKI_SUBMODULE_PATH}/{file_name}");

    let h2_headings: Vec<&MarkdownHeadingRecord> =
        headings.iter().filter(|h| h.level == 2).collect();
    let h2_normalized: Vec<String> = h2_headings
        .iter()
        .map(|h| normalize_heading(&h.text))
        .collect();
    let expected_h2: Vec<String> = WIKI_INSTRUCTIONAL_SECTION_SEQUENCE
        .iter()
        .map(|h| normalize_heading(h))
        .collect();
    if h2_normalized != expected_h2 {
        let line = h2_headings.first().map(|h| h.line);
        let found = if h2_headings.is_empty() {
            "<none>".to_string()
        } else {
            h2_headings
                .iter()
                .map(|h| format!("{}@L{}", h.text, h.line))
                .collect::<Vec<_>>()
                .join(", ")
        };
        problems.push(Problem::new(
            "wiki",
            path.clone(),
            format!(
                "instructional page must use exact level-2 section sequence `{}` (found `{found}`)",
                WIKI_INSTRUCTIONAL_SECTION_SEQUENCE.join("`, `")
            ),
            line,
        ));
    }

    let entry_idx = headings.iter().position(|h| {
        h.level == 2 && normalize_heading(&h.text) == normalize_heading("Entry Criteria")
    });
    let procedure_idx = headings
        .iter()
        .position(|h| h.level == 2 && normalize_heading(&h.text) == normalize_heading("Procedure"));

    if let Some(entry_idx) = entry_idx {
        let boundary = procedure_idx.unwrap_or(headings.len());
        let entry_subsections: Vec<&MarkdownHeadingRecord> = headings[entry_idx + 1..boundary]
            .iter()
            .filter(|h| h.level == 3)
            .collect();
        let entry_subsection_names: Vec<String> = entry_subsections
            .iter()
            .map(|h| normalize_heading(&h.text))
            .collect();
        let expected_entry_subsections: Vec<String> = WIKI_INSTRUCTIONAL_ENTRY_CRITERIA_SUBSECTIONS
            .iter()
            .map(|h| normalize_heading(h))
            .collect();
        if entry_subsection_names != expected_entry_subsections {
            let found = if entry_subsections.is_empty() {
                "<none>".to_string()
            } else {
                entry_subsections
                    .iter()
                    .map(|h| format!("{}@L{}", h.text, h.line))
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            problems.push(Problem::new(
                "wiki",
                path,
                format!(
                    "`## Entry Criteria` must contain exact level-3 subsection sequence `{}` before `## Procedure` (found `{found}`)",
                    WIKI_INSTRUCTIONAL_ENTRY_CRITERIA_SUBSECTIONS.join("`, `")
                ),
                Some(headings[entry_idx].line),
            ));
        }
    }

    problems
}

fn extract_markdown_heading_records(body: &str) -> Vec<MarkdownHeadingRecord> {
    let mut headings = Vec::new();
    let mut in_fence = false;

    for (line_idx, line) in body.lines().enumerate() {
        if parse_fence_lang(line).is_some() {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let Some((level, heading_text)) = parse_markdown_heading_parts(line) else {
            continue;
        };
        headings.push(MarkdownHeadingRecord {
            level,
            text: heading_text.trim().to_string(),
            line: line_idx + 1,
        });
    }

    headings
}

fn parse_gitmodules_section(text: &str, target_section: &str) -> Option<HashMap<String, String>> {
    let mut current_section: Option<String> = None;
    let mut values = HashMap::new();
    let mut found = false;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') && line.len() >= 2 {
            let name = &line[1..line.len() - 1];
            current_section = Some(name.to_string());
            if name == target_section {
                found = true;
            }
            continue;
        }
        if current_section.as_deref() != Some(target_section) {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        values.insert(key.trim().to_string(), value.trim().to_string());
    }

    found.then_some(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wiki_instructional_template_accepts_standard_page() {
        let text = r#"# Tutorial: Example

## Outcome

Do a thing.

## Entry Criteria

### Prior Knowledge

Know basics.

### Environment Setup

Have cargo.

### Dependencies

Have wiki checked out.

## Procedure

Do the thing.

## Validation

Observe the result.

## Next Steps

Read the next page.
"#;
        let problems = validate_wiki_instructional_page_template("Tutorial-Example.md", text);
        assert!(problems.is_empty(), "{problems:?}");
    }

    #[test]
    fn wiki_instructional_template_rejects_h2_sequence_drift() {
        let text = r#"# Tutorial: Example

## Entry Criteria
## Outcome
## Procedure
## Validation
## Next Steps
"#;
        let problems = validate_wiki_instructional_page_template("Tutorial-Example.md", text);
        assert!(problems
            .iter()
            .any(|p| p.message.contains("level-2 section sequence")));
    }

    #[test]
    fn wiki_instructional_template_rejects_entry_criteria_subsection_drift() {
        let text = r#"# How-to: Example

## Outcome
## Entry Criteria
### Environment Setup
### Prior Knowledge
### Dependencies
## Procedure
## Validation
## Next Steps
"#;
        let problems = validate_wiki_instructional_page_template("How-to-Example.md", text);
        assert!(problems
            .iter()
            .any(|p| p.message.contains("level-3 subsection sequence")));
    }
}
