use super::*;
use crate::wiki_config::resolve_wiki_checkout;

const WIKI_CONFIG_PATH: &str = "tools/docs/wiki.toml";

pub(crate) fn validate_wiki_checkout(root: &Path, require_checkout: bool) -> Vec<Problem> {
    let mut problems = Vec::new();
    let checkout = match resolve_wiki_checkout(root) {
        Ok(checkout) => checkout,
        Err(err) => {
            problems.push(Problem::new(
                "wiki",
                WIKI_CONFIG_PATH,
                err.to_string(),
                None,
            ));
            return problems;
        }
    };

    let wiki_root = checkout.path;
    if !wiki_root.exists() {
        if require_checkout {
            problems.push(Problem::new(
                "wiki",
                WIKI_CONFIG_PATH,
                format!(
                    "external wiki checkout is missing at `{}`; run `cargo wiki clone` or set `OS_WIKI_PATH`",
                    wiki_root.display()
                ),
                None,
            ));
        }
        return problems;
    }

    if !wiki_root.join(".git").exists() {
        problems.push(Problem::new(
            "wiki",
            wiki_root.display().to_string(),
            "expected external wiki checkout to be a git working tree (`.git` missing)",
            None,
        ));
        return problems;
    }

    for page in REQUIRED_WIKI_PAGES {
        let page_path = wiki_root.join(page);
        if !page_path.exists() {
            problems.push(Problem::new(
                "wiki",
                page_path.display().to_string(),
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
                home_path.display().to_string(),
                "Home page should describe the Diataxis organization of the wiki",
                None,
            ));
        }
        if !home_text.to_lowercase().contains("rustdoc") {
            problems.push(Problem::new(
                "wiki",
                home_path.display().to_string(),
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
                os_wiki_path.display().to_string(),
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
                    sidebar_path.display().to_string(),
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
                wiki_root.display().to_string(),
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
                    path.display().to_string(),
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
    let path = file_name.to_string();

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
        let boundary = procedure_idx
            .filter(|idx| *idx > entry_idx)
            .unwrap_or(headings.len());
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
                    "Entry Criteria must use exact level-3 subsection sequence `{}` (found `{found}`)",
                    WIKI_INSTRUCTIONAL_ENTRY_CRITERIA_SUBSECTIONS.join("`, `")
                ),
                entry_subsections.first().map(|h| h.line),
            ));
        }
    }

    problems
}

fn extract_markdown_heading_records(text: &str) -> Vec<MarkdownHeadingRecord> {
    let mut headings = Vec::new();
    for (idx, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let level = line.chars().take_while(|c| *c == '#').count();
        if level == 0 || level > 6 {
            continue;
        }
        let text = rest.trim_start_matches('#').trim();
        if text.is_empty() {
            continue;
        }
        headings.push(MarkdownHeadingRecord {
            level,
            text: text.to_string(),
            line: idx + 1,
        });
    }
    headings
}

fn normalize_heading(text: &str) -> String {
    text.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wiki_instructional_template_accepts_standard_page() {
        let text = r#"
## Outcome

## Entry Criteria

### Prior Knowledge

### Environment Setup

### Dependencies

## Procedure

## Validation

## Next Steps
"#;
        let problems = validate_wiki_instructional_page_template("Tutorial-Example.md", text);
        assert!(problems.is_empty(), "{problems:#?}");
    }

    #[test]
    fn wiki_instructional_template_rejects_h2_sequence_drift() {
        let text = r#"
## Outcome

## Procedure

## Entry Criteria

### Prior Knowledge

### Environment Setup

### Dependencies

## Validation

## Next Steps
"#;
        let problems = validate_wiki_instructional_page_template("Tutorial-Example.md", text);
        assert!(!problems.is_empty());
    }

    #[test]
    fn wiki_instructional_template_rejects_entry_criteria_subsection_drift() {
        let text = r#"
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
        assert!(!problems.is_empty());
    }
}
