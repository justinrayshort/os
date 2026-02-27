use chrono::{Local, NaiveDate, SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

const FRONTMATTER_DELIM: &str = "---";
const WIKI_SUBMODULE_PATH: &str = "wiki";
const WIKI_SUBMODULE_URL: &str = "https://github.com/justinrayshort/os.wiki.git";
const REQUIRED_WIKI_PAGES: &[&str] = &[
    "Home.md",
    "OS-Wiki.md",
    "_Sidebar.md",
    "Tutorials.md",
    "How-To-Guides.md",
    "Explanations.md",
    "API-Reference-(rustdoc).md",
    "Contributing-to-Docs.md",
];
const WIKI_INSTRUCTIONAL_SECTION_SEQUENCE: &[&str] = &[
    "Outcome",
    "Entry Criteria",
    "Procedure",
    "Validation",
    "Next Steps",
];
const WIKI_INSTRUCTIONAL_ENTRY_CRITERIA_SUBSECTIONS: &[&str] =
    &["Prior Knowledge", "Environment Setup", "Dependencies"];

#[derive(Clone, Debug)]
struct Problem {
    check: String,
    path: String,
    message: String,
    line: Option<usize>,
}

impl Problem {
    fn new(
        check: &str,
        path: impl Into<String>,
        message: impl Into<String>,
        line: Option<usize>,
    ) -> Self {
        Self {
            check: check.to_string(),
            path: path.into(),
            message: message.into(),
            line,
        }
    }
}

#[derive(Clone, Debug)]
struct LinkRef {
    target: String,
    line: usize,
}

#[derive(Clone, Debug)]
struct DocRecord {
    path: PathBuf,
    rel_path: String,
    frontmatter: Map<String, Value>,
    body: String,
    headings: Vec<String>,
    anchors: HashSet<String>,
    links: Vec<LinkRef>,
}

#[derive(Debug, Deserialize)]
struct Contracts {
    required_frontmatter: Vec<String>,
    allowed_categories: Vec<String>,
    allowed_statuses: Vec<String>,
    allowed_owners: Vec<String>,
    diataxis_category_by_folder: BTreeMap<String, String>,
    required_docs_directories: Vec<String>,
    root_docs_allowed_categories: Vec<String>,
    stale_review_days: u64,
    sop_required_headings: Vec<String>,
}

#[derive(Clone, Debug, Default)]
struct DocsFlags {
    require_renderer: bool,
    require_openapi_validator: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DocsCommand {
    Structure,
    Wiki,
    Frontmatter,
    Sop,
    Links,
    Mermaid,
    OpenApi,
    UiConformance,
    StorageBoundary,
    All,
    AuditReport,
}

pub(crate) fn print_docs_usage() {
    eprintln!(
        "Usage: cargo xtask docs <command> [args]\n\
         \n\
         Commands:\n\
           structure                         Validate required docs directory structure\n\
           wiki                              Validate wiki submodule wiring and required pages\n\
           frontmatter                       Validate docs frontmatter contracts and Diataxis mapping\n\
           sop                               Validate SOP required heading order\n\
           links                             Validate internal markdown links and anchors\n\
           mermaid [--require-renderer]      Validate Mermaid blocks/files (structural checks)\n\
           openapi [--require-validator]     Validate OpenAPI specs (Rust-native parse/sanity)\n\
           ui-conformance                    Validate machine-checkable UI conformance token/literal rules\n\
           storage-boundary                  Enforce typed app-state load boundary in app/runtime crates\n\
           all [flags]                       Run all docs checks\n\
             Flags: --require-renderer --require-openapi-validator\n\
           audit-report --output <path>      Write docs audit report JSON and fail on validation issues\n"
    );
}

pub(crate) fn run_docs_command(root: &Path, args: Vec<String>) -> Result<(), String> {
    let Some(command) = args.first().map(String::as_str) else {
        print_docs_usage();
        return Ok(());
    };

    if matches!(command, "help" | "--help" | "-h") {
        print_docs_usage();
        return Ok(());
    }

    let (cmd, flags, audit_output) = parse_docs_command(&args)?;

    let contracts = load_contracts(root)?;
    let (records, parse_problems) = collect_docs(root)?;

    match cmd {
        DocsCommand::Structure => fail_if_problems(validate_structure(root, &contracts)),
        DocsCommand::Wiki => fail_if_problems(validate_wiki_submodule(root)),
        DocsCommand::Frontmatter => {
            let mut problems = parse_problems;
            problems.extend(validate_frontmatter(root, &records, &contracts));
            fail_if_problems(problems)
        }
        DocsCommand::Sop => fail_if_problems(validate_sop_headings(root, &records, &contracts)),
        DocsCommand::Links => fail_if_problems(validate_links(root, &records)),
        DocsCommand::Mermaid => {
            let (problems, count) = validate_mermaid(root, &records, flags.require_renderer);
            println!("Mermaid sources checked: {count}");
            fail_if_problems(problems)
        }
        DocsCommand::OpenApi => {
            let (problems, count) = validate_openapi(root, flags.require_openapi_validator);
            println!("OpenAPI specs checked: {count}");
            fail_if_problems(problems)
        }
        DocsCommand::UiConformance => fail_if_problems(validate_ui_conformance(root)),
        DocsCommand::StorageBoundary => fail_if_problems(validate_typed_persistence_boundary(root)),
        DocsCommand::All => {
            let problems = run_all(root, &records, parse_problems, &contracts, &flags);
            fail_if_problems(problems)
        }
        DocsCommand::AuditReport => {
            let output = audit_output.ok_or_else(|| "missing `--output <path>`".to_string())?;
            write_audit_report(root, &records, parse_problems, &contracts, &output)
        }
    }
}

fn parse_docs_command(
    args: &[String],
) -> Result<(DocsCommand, DocsFlags, Option<PathBuf>), String> {
    let mut flags = DocsFlags::default();
    let mut output: Option<PathBuf> = None;

    let cmd = match args[0].as_str() {
        "structure" => DocsCommand::Structure,
        "wiki" => DocsCommand::Wiki,
        "frontmatter" => DocsCommand::Frontmatter,
        "sop" => DocsCommand::Sop,
        "links" => DocsCommand::Links,
        "mermaid" => DocsCommand::Mermaid,
        "openapi" => DocsCommand::OpenApi,
        "ui-conformance" => DocsCommand::UiConformance,
        "storage-boundary" => DocsCommand::StorageBoundary,
        "all" => DocsCommand::All,
        "audit-report" => DocsCommand::AuditReport,
        other => return Err(format!("unsupported docs command: {other}")),
    };

    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--require-renderer" => {
                flags.require_renderer = true;
                i += 1;
            }
            "--require-validator" => {
                flags.require_openapi_validator = true;
                i += 1;
            }
            "--require-openapi-validator" => {
                flags.require_openapi_validator = true;
                i += 1;
            }
            "--output" => {
                let Some(path) = args.get(i + 1) else {
                    return Err("missing value for `--output`".to_string());
                };
                output = Some(PathBuf::from(path));
                i += 2;
            }
            other => return Err(format!("unsupported docs argument: {other}")),
        }
    }

    match cmd {
        DocsCommand::Structure
        | DocsCommand::Wiki
        | DocsCommand::Frontmatter
        | DocsCommand::Sop
        | DocsCommand::Links
        | DocsCommand::StorageBoundary => {
            if flags.require_renderer || flags.require_openapi_validator || output.is_some() {
                return Err(format!(
                    "extra arguments are not supported for `{}`",
                    args[0]
                ));
            }
        }
        DocsCommand::Mermaid => {
            if flags.require_openapi_validator || output.is_some() {
                return Err("`mermaid` only supports `--require-renderer`".to_string());
            }
        }
        DocsCommand::OpenApi => {
            if flags.require_renderer || output.is_some() {
                return Err("`openapi` only supports `--require-validator`".to_string());
            }
        }
        DocsCommand::UiConformance => {
            if flags.require_renderer || flags.require_openapi_validator || output.is_some() {
                return Err("`ui-conformance` does not accept extra arguments".to_string());
            }
        }
        DocsCommand::AuditReport => {
            if flags.require_renderer || flags.require_openapi_validator {
                return Err("`audit-report` does not accept validator flags".to_string());
            }
            if output.is_none() {
                return Err("`audit-report` requires `--output <path>`".to_string());
            }
        }
        DocsCommand::All => {}
    }

    Ok((cmd, flags, output))
}

fn docs_root(root: &Path) -> PathBuf {
    root.join("docs")
}

fn load_contracts(root: &Path) -> Result<Contracts, String> {
    let path = root.join("tools/docs/doc_contracts.json");
    let text = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn collect_docs(root: &Path) -> Result<(Vec<DocRecord>, Vec<Problem>), String> {
    let mut records = Vec::new();
    let mut problems = Vec::new();
    let mut files = collect_files_with_suffix(&docs_root(root), ".md")?;
    files.sort();

    for path in files {
        let text = fs::read_to_string(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        let (frontmatter, body, fm_errors) = split_frontmatter(&text);
        let rel = rel_posix(root, &path);
        for err in fm_errors {
            problems.push(Problem::new("frontmatter", rel.clone(), err, None));
        }
        let (headings, anchors) = extract_headings_and_anchors(&body);
        let links = extract_links(&body);
        records.push(DocRecord {
            path: normalize_path(&path),
            rel_path: rel,
            frontmatter,
            body,
            headings,
            anchors,
            links,
        });
    }

    Ok((records, problems))
}

fn collect_files_with_suffix(root: &Path, suffix: &str) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    collect_files_with_suffix_inner(root, suffix, &mut out)?;
    Ok(out)
}

fn collect_files_with_suffix_inner(
    root: &Path,
    suffix: &str,
    out: &mut Vec<PathBuf>,
) -> Result<(), String> {
    let mut entries: Vec<_> = fs::read_dir(root)
        .map_err(|err| format!("failed to read {}: {err}", root.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read {}: {err}", root.display()))?;
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_files_with_suffix_inner(&path, suffix, out)?;
        } else if path.is_file() && path.to_string_lossy().ends_with(suffix) {
            out.push(path);
        }
    }

    Ok(())
}

fn split_frontmatter(text: &str) -> (Map<String, Value>, String, Vec<String>) {
    if !text.starts_with(FRONTMATTER_DELIM) {
        return (
            Map::new(),
            text.to_string(),
            vec!["missing frontmatter start delimiter".to_string()],
        );
    }

    let lines: Vec<&str> = text.lines().collect();
    if lines.first().map(|l| l.trim()) != Some(FRONTMATTER_DELIM) {
        return (
            Map::new(),
            text.to_string(),
            vec!["invalid frontmatter start delimiter".to_string()],
        );
    }

    let end_idx = lines
        .iter()
        .enumerate()
        .skip(1)
        .find_map(|(i, line)| (line.trim() == FRONTMATTER_DELIM).then_some(i));

    let Some(end_idx) = end_idx else {
        return (
            Map::new(),
            text.to_string(),
            vec!["missing frontmatter end delimiter".to_string()],
        );
    };

    let raw_frontmatter = lines[1..end_idx].join("\n");
    let body = lines[end_idx + 1..].join("\n");
    let (fm, errors) = parse_yamlish_frontmatter(&raw_frontmatter);
    (fm, body, errors)
}

fn parse_yamlish_frontmatter(raw: &str) -> (Map<String, Value>, Vec<String>) {
    let mut data = Map::new();
    let mut errors = Vec::new();
    let mut current_list_key: Option<String> = None;

    for (idx, raw_line) in raw.lines().enumerate() {
        let line_num = idx + 1;
        let line = raw_line.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        let trimmed_start = line.trim_start();
        if trimmed_start.starts_with('#') {
            continue;
        }

        if let Some(item_text) = parse_list_item_line(trimmed_start) {
            let Some(key) = current_list_key.as_ref() else {
                errors.push(format!(
                    "frontmatter line {line_num}: list item without list key"
                ));
                continue;
            };
            match data.get_mut(key) {
                Some(Value::Array(items)) => {
                    items.push(parse_scalar(item_text));
                }
                _ => {
                    errors.push(format!(
                        "frontmatter line {line_num}: list item without list key"
                    ));
                }
            }
            continue;
        }

        let Some((key, value)) = parse_frontmatter_key_value(line) else {
            errors.push(format!(
                "frontmatter line {line_num}: unsupported syntax `{line}`"
            ));
            current_list_key = None;
            continue;
        };

        if value.is_empty() {
            data.insert(key.to_string(), Value::Array(Vec::new()));
            current_list_key = Some(key.to_string());
        } else if value.starts_with('[') && value.ends_with(']') {
            let list = parse_inline_list(value);
            data.insert(key.to_string(), Value::Array(list));
            current_list_key = None;
        } else {
            data.insert(key.to_string(), parse_scalar(value));
            current_list_key = None;
        }
    }

    (data, errors)
}

fn parse_list_item_line(trimmed_start: &str) -> Option<&str> {
    trimmed_start.strip_prefix("- ").map(str::trim)
}

fn parse_frontmatter_key_value(line: &str) -> Option<(&str, &str)> {
    let (key, value) = line.split_once(':')?;
    if key.is_empty()
        || !key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    Some((key, value.trim_start()))
}

fn parse_scalar(value: &str) -> Value {
    let value = value.trim();
    if value.is_empty() {
        return Value::String(String::new());
    }
    if value.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }
    if value.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }
    Value::String(strip_quotes(value).to_string())
}

fn strip_quotes(value: &str) -> &str {
    let value = value.trim();
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
        {
            return &value[1..value.len() - 1];
        }
    }
    value
}

fn parse_inline_list(value: &str) -> Vec<Value> {
    if value.len() < 2 {
        return Vec::new();
    }
    let inner = &value[1..value.len() - 1];
    let inner = inner.trim();
    if inner.is_empty() {
        return Vec::new();
    }

    let mut items: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut quote: Option<char> = None;

    for ch in inner.chars() {
        if let Some(q) = quote {
            if ch == q {
                quote = None;
            }
            buf.push(ch);
            continue;
        }
        if ch == '\'' || ch == '"' {
            quote = Some(ch);
            buf.push(ch);
            continue;
        }
        if ch == ',' {
            if !buf.trim().is_empty() {
                items.push(buf.trim().to_string());
            }
            buf.clear();
            continue;
        }
        buf.push(ch);
    }

    if !buf.trim().is_empty() {
        items.push(buf.trim().to_string());
    }

    items.into_iter().map(|s| parse_scalar(&s)).collect()
}

fn extract_headings_and_anchors(body: &str) -> (Vec<String>, HashSet<String>) {
    let mut headings = Vec::new();
    let mut anchors = HashSet::new();
    let mut counts: HashMap<String, usize> = HashMap::new();
    let mut in_fence = false;

    for line in body.lines() {
        if parse_fence_lang(line).is_some() {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let Some(heading_text) = parse_markdown_heading(line) else {
            continue;
        };
        let heading_text = heading_text.trim().to_string();
        headings.push(heading_text.clone());

        let base = slugify_heading(&heading_text);
        if base.is_empty() {
            continue;
        }
        let count = counts.entry(base.clone()).or_insert(0);
        let anchor = if *count == 0 {
            base
        } else {
            format!("{base}-{}", *count)
        };
        *count += 1;
        anchors.insert(anchor);
    }

    (headings, anchors)
}

fn parse_fence_lang(line: &str) -> Option<String> {
    if !line.starts_with("```") {
        return None;
    }
    let rest = &line[3..];
    let mut lang = String::new();
    let mut seen_ws = false;
    for ch in rest.chars() {
        if ch.is_ascii_whitespace() {
            seen_ws = true;
            continue;
        }
        if seen_ws {
            return None;
        }
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            lang.push(ch);
        } else {
            return None;
        }
    }
    Some(lang)
}

fn parse_markdown_heading(line: &str) -> Option<&str> {
    parse_markdown_heading_parts(line).map(|(_, text)| text)
}

fn parse_markdown_heading_parts(line: &str) -> Option<(usize, &str)> {
    let bytes = line.as_bytes();
    let mut count = 0usize;
    while count < bytes.len() && bytes[count] == b'#' {
        count += 1;
    }
    if count == 0 || count > 6 {
        return None;
    }
    if bytes.get(count).copied().map(|b| b.is_ascii_whitespace()) != Some(true) {
        return None;
    }
    Some((count, line[count..].trim_start()))
}

fn slugify_heading(text: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;

    for ch in text.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch == '`' {
            continue;
        }
        let keep = ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch.is_ascii_whitespace();
        if !keep {
            continue;
        }
        if ch.is_ascii_whitespace() {
            if !out.is_empty() && !prev_dash {
                out.push('-');
                prev_dash = true;
            }
            continue;
        }
        if ch == '-' {
            if out.is_empty() || prev_dash {
                continue;
            }
            out.push(ch);
            prev_dash = true;
            continue;
        }
        out.push(ch);
        prev_dash = false;
    }

    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn extract_links(body: &str) -> Vec<LinkRef> {
    let mut links = Vec::new();
    let mut in_fence = false;

    for (line_no, line) in body.lines().enumerate() {
        let line_no = line_no + 1;
        if parse_fence_lang(line).is_some() {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        for target in extract_markdown_link_targets_from_line(line) {
            links.push(LinkRef {
                target,
                line: line_no,
            });
        }
    }

    links
}

fn extract_markdown_link_targets_from_line(line: &str) -> Vec<String> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'[' || (i > 0 && bytes[i - 1] == b'!') {
            i += 1;
            continue;
        }

        let Some(label_end) = find_byte(bytes, b']', i + 1) else {
            break;
        };
        if bytes.get(label_end + 1) != Some(&b'(') {
            i = label_end + 1;
            continue;
        }
        let Some(target_end) = find_byte(bytes, b')', label_end + 2) else {
            break;
        };

        let raw_target = &line[label_end + 2..target_end];
        let mut target = raw_target.trim().to_string();
        if target.contains(' ') && !target.starts_with('<') {
            if let Some((before, _)) = target.split_once(' ') {
                target = before.to_string();
            }
        }
        if target.starts_with('<') && target.ends_with('>') && target.len() >= 2 {
            target = target[1..target.len() - 1].to_string();
        }
        out.push(target);
        i = target_end + 1;
    }

    out
}

fn find_byte(bytes: &[u8], target: u8, start: usize) -> Option<usize> {
    bytes[start..]
        .iter()
        .position(|b| *b == target)
        .map(|offset| start + offset)
}

fn validate_structure(root: &Path, contracts: &Contracts) -> Vec<Problem> {
    let mut problems = Vec::new();
    let docs_root = docs_root(root);
    for dirname in &contracts.required_docs_directories {
        let path = docs_root.join(dirname);
        if !path.exists() {
            problems.push(Problem::new(
                "structure",
                format!("docs/{dirname}"),
                "required directory is missing",
                None,
            ));
        }
    }
    problems
}

fn validate_wiki_submodule(root: &Path) -> Vec<Problem> {
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

fn validate_wiki_instructional_page_template(file_name: &str, text: &str) -> Vec<Problem> {
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

fn parse_review_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

fn current_date() -> Result<NaiveDate, String> {
    if let Ok(override_date) = env::var("DOCS_TODAY") {
        return parse_review_date(&override_date).ok_or_else(|| {
            format!("invalid DOCS_TODAY date `{override_date}` (expected YYYY-MM-DD)")
        });
    }
    Ok(Local::now().date_naive())
}

fn env_or_contract_stale_days(contracts: &Contracts) -> Result<i64, String> {
    if let Ok(value) = env::var("DOCS_STALE_REVIEW_DAYS") {
        let parsed = value
            .parse::<i64>()
            .map_err(|_| format!("invalid DOCS_STALE_REVIEW_DAYS `{value}`"))?;
        return Ok(parsed);
    }
    Ok(contracts.stale_review_days as i64)
}

fn validate_frontmatter(root: &Path, records: &[DocRecord], contracts: &Contracts) -> Vec<Problem> {
    let mut problems = Vec::new();
    let allowed_categories: HashSet<&str> = contracts
        .allowed_categories
        .iter()
        .map(String::as_str)
        .collect();
    let allowed_statuses: HashSet<&str> = contracts
        .allowed_statuses
        .iter()
        .map(String::as_str)
        .collect();
    let allowed_owners: HashSet<&str> = contracts
        .allowed_owners
        .iter()
        .map(String::as_str)
        .collect();
    let root_allowed: HashSet<&str> = contracts
        .root_docs_allowed_categories
        .iter()
        .map(String::as_str)
        .collect();
    let stale_days =
        env_or_contract_stale_days(contracts).unwrap_or(contracts.stale_review_days as i64);
    let today = current_date().unwrap_or_else(|_| Local::now().date_naive());
    let docs_root = docs_root(root);

    for record in records {
        let fm = &record.frontmatter;
        for field in &contracts.required_frontmatter {
            if !fm.contains_key(field) {
                problems.push(Problem::new(
                    "frontmatter",
                    record.rel_path.clone(),
                    format!("missing required field `{field}`"),
                    None,
                ));
            }
        }

        if fm.is_empty() {
            continue;
        }

        let title = fm.get("title").and_then(Value::as_str);
        if title.is_none_or(|s| s.trim().is_empty()) {
            problems.push(Problem::new(
                "frontmatter",
                record.rel_path.clone(),
                "`title` must be a non-empty string",
                None,
            ));
        }

        let category = fm.get("category").and_then(Value::as_str);
        if category.is_none_or(|v| !allowed_categories.contains(v)) {
            problems.push(Problem::new(
                "frontmatter",
                record.rel_path.clone(),
                format!(
                    "`category` must be one of {:?}",
                    sorted_strings(&contracts.allowed_categories)
                ),
                None,
            ));
        }

        let owner = fm.get("owner").and_then(Value::as_str);
        if owner.is_none_or(|v| !allowed_owners.contains(v)) {
            problems.push(Problem::new(
                "frontmatter",
                record.rel_path.clone(),
                format!(
                    "`owner` must be one of {:?}",
                    sorted_strings(&contracts.allowed_owners)
                ),
                None,
            ));
        }

        let status = fm.get("status").and_then(Value::as_str);
        if status.is_none_or(|v| !allowed_statuses.contains(v)) {
            problems.push(Problem::new(
                "frontmatter",
                record.rel_path.clone(),
                format!(
                    "`status` must be one of {:?}",
                    sorted_strings(&contracts.allowed_statuses)
                ),
                None,
            ));
        }
        if status == Some("superseded") && fm.get("superseded_by").and_then(Value::as_str).is_none()
        {
            problems.push(Problem::new(
                "frontmatter",
                record.rel_path.clone(),
                "`superseded` docs must declare `superseded_by`",
                None,
            ));
        }

        match fm.get("last_reviewed").and_then(Value::as_str) {
            None => problems.push(Problem::new(
                "frontmatter",
                record.rel_path.clone(),
                "`last_reviewed` must be an ISO date string",
                None,
            )),
            Some(reviewed) => match parse_review_date(reviewed) {
                None => problems.push(Problem::new(
                    "frontmatter",
                    record.rel_path.clone(),
                    "`last_reviewed` is not a valid ISO date",
                    None,
                )),
                Some(review_date) => {
                    let age_days = (today - review_date).num_days();
                    if age_days > stale_days {
                        problems.push(Problem::new(
                            "frontmatter",
                            record.rel_path.clone(),
                            format!("`last_reviewed` is stale ({age_days} days > {stale_days})"),
                            None,
                        ));
                    }
                }
            },
        }

        for list_field in ["audience", "invariants"] {
            match fm.get(list_field) {
                Some(Value::Array(items)) if !items.is_empty() => {
                    let valid = items
                        .iter()
                        .all(|v| v.as_str().is_some_and(|s| !s.trim().is_empty()));
                    if !valid {
                        problems.push(Problem::new(
                            "frontmatter",
                            record.rel_path.clone(),
                            format!("`{list_field}` must contain non-empty strings"),
                            None,
                        ));
                    }
                }
                _ => problems.push(Problem::new(
                    "frontmatter",
                    record.rel_path.clone(),
                    format!("`{list_field}` must be a non-empty list"),
                    None,
                )),
            }
        }

        let rel_doc = match record.path.strip_prefix(&docs_root) {
            Ok(path) => path,
            Err(_) => continue,
        };
        let parts: Vec<_> = rel_doc.components().collect();
        if parts.len() == 1 {
            if let Some(category) = category {
                if !root_allowed.contains(category) {
                    problems.push(Problem::new(
                        "diataxis",
                        record.rel_path.clone(),
                        format!(
                            "root docs page category `{category}` not allowed; expected one of {:?}",
                            sorted_strings(&contracts.root_docs_allowed_categories)
                        ),
                        None,
                    ));
                }
            }
        } else if let Some(Component::Normal(folder_os)) = parts.first() {
            let folder = folder_os.to_string_lossy();
            if let Some(expected) = contracts.diataxis_category_by_folder.get(folder.as_ref()) {
                if category != Some(expected.as_str()) {
                    let category_str = category.unwrap_or("");
                    problems.push(Problem::new(
                        "diataxis",
                        record.rel_path.clone(),
                        format!(
                            "category `{category_str}` does not match folder `{}` -> `{expected}`",
                            folder
                        ),
                        None,
                    ));
                }
            }
            if folder == "adr"
                && !is_valid_adr_filename(
                    &record
                        .path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy(),
                )
            {
                problems.push(Problem::new(
                    "adr",
                    record.rel_path.clone(),
                    "ADR filename must match ADR-0000-name.md",
                    None,
                ));
            }
        }
    }

    problems
}

fn sorted_strings(values: &[String]) -> Vec<String> {
    let mut v = values.to_vec();
    v.sort();
    v
}

fn is_valid_adr_filename(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".md") else {
        return false;
    };
    let Some(rest) = stem.strip_prefix("ADR-") else {
        return false;
    };
    if rest.len() < 4 {
        return false;
    }
    let (digits, suffix) = rest.split_at(4);
    if !digits.chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    suffix
        .chars()
        .all(|c| c == '-' || c == '_' || c.is_ascii_alphanumeric())
}

fn validate_sop_headings(
    root: &Path,
    records: &[DocRecord],
    contracts: &Contracts,
) -> Vec<Problem> {
    let required: Vec<String> = contracts
        .sop_required_headings
        .iter()
        .map(|h| normalize_heading(h))
        .collect();
    let docs_root = docs_root(root);
    let mut problems = Vec::new();

    for record in records {
        let rel_doc = match record.path.strip_prefix(&docs_root) {
            Ok(path) => path,
            Err(_) => continue,
        };
        let mut components = rel_doc.components();
        let Some(Component::Normal(folder)) = components.next() else {
            continue;
        };
        if folder != "sop" {
            continue;
        }

        let headings: Vec<String> = record
            .headings
            .iter()
            .map(|h| normalize_heading(h))
            .collect();
        let mut pos = 0usize;
        for req in &required {
            match headings[pos..].iter().position(|h| h == req) {
                Some(found_rel) => {
                    pos += found_rel + 1;
                }
                None => {
                    problems.push(Problem::new(
                        "sop",
                        record.rel_path.clone(),
                        format!("missing or out-of-order heading `{req}`"),
                        None,
                    ));
                    break;
                }
            }
        }
    }

    problems
}

fn normalize_heading(value: &str) -> String {
    collapse_whitespace(value.trim())
}

fn collapse_whitespace(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_space = false;
    for ch in value.chars() {
        if ch.is_whitespace() {
            if !out.is_empty() && !last_was_space {
                out.push(' ');
            }
            last_was_space = true;
        } else {
            out.push(ch);
            last_was_space = false;
        }
    }
    out.trim_end().to_string()
}

fn validate_links(root: &Path, records: &[DocRecord]) -> Vec<Problem> {
    let mut problems = Vec::new();
    let heading_cache: HashMap<PathBuf, &HashSet<String>> = records
        .iter()
        .map(|r| (normalize_path(&r.path), &r.anchors))
        .collect();

    for record in records {
        for link in &record.links {
            let (resolved, anchor, skip_reason) = resolve_markdown_link(root, record, &link.target);
            if skip_reason.is_some() {
                continue;
            }
            let Some(resolved) = resolved else {
                continue;
            };
            let resolved_key = normalize_path(&resolved);
            if !resolved_key.exists() {
                problems.push(Problem::new(
                    "links",
                    record.rel_path.clone(),
                    format!(
                        "broken link target `{}` (resolved `{}`)",
                        link.target,
                        resolved_key.display()
                    ),
                    Some(link.line),
                ));
                continue;
            }
            if let Some(anchor) = anchor {
                if resolved_key
                    .extension()
                    .and_then(|e| e.to_str())
                    .is_some_and(|e| e.eq_ignore_ascii_case("md"))
                {
                    if let Some(anchors) = heading_cache.get(&resolved_key) {
                        if !anchors.contains(&anchor) {
                            let rel_target = rel_posix(root, &resolved_key);
                            problems.push(Problem::new(
                                "links",
                                record.rel_path.clone(),
                                format!("missing anchor `#{anchor}` in `{rel_target}`"),
                                Some(link.line),
                            ));
                        }
                    }
                }
            }
        }
    }

    problems
}

fn resolve_markdown_link(
    root: &Path,
    record: &DocRecord,
    target: &str,
) -> (Option<PathBuf>, Option<String>, Option<&'static str>) {
    if target.is_empty() || target.starts_with('?') {
        return (None, None, Some("query-only link"));
    }
    if has_uri_scheme(target) {
        return (None, None, Some("external link"));
    }
    if target.starts_with("//") {
        return (None, None, Some("network-path link"));
    }

    let (path_part, anchor) = split_link_target(target);

    if !path_part.is_empty() && path_part.contains('?') {
        return (None, anchor, Some("route/query link"));
    }

    if path_part.starts_with('/') && !path_part.starts_with("/docs/") {
        return (None, anchor, Some("site route"));
    }

    let resolved = if path_part.is_empty() {
        record.path.clone()
    } else if path_part.starts_with("/docs/") || path_part.starts_with('/') {
        normalize_path(&root.join(path_part.trim_start_matches('/')))
    } else {
        let parent = record
            .path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| root.to_path_buf());
        normalize_path(&parent.join(path_part))
    };

    if resolved.exists() {
        return (Some(resolved), anchor, None);
    }

    if resolved.extension().is_none() {
        let md_candidate = resolved.with_extension("md");
        if md_candidate.exists() {
            return (Some(md_candidate), anchor, None);
        }
        let index_candidate = resolved.join("index.md");
        if index_candidate.exists() {
            return (Some(index_candidate), anchor, None);
        }
    }

    (Some(resolved), anchor, None)
}

fn split_link_target(target: &str) -> (&str, Option<String>) {
    match target.split_once('#') {
        Some((path, anchor)) => {
            let anchor = if anchor.is_empty() {
                None
            } else {
                Some(anchor.to_string())
            };
            (path, anchor)
        }
        None => (target, None),
    }
}

fn has_uri_scheme(target: &str) -> bool {
    let Some((scheme, _)) = target.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
}

fn extract_fenced_mermaid_blocks(record: &DocRecord) -> (Vec<(usize, String)>, Vec<Problem>) {
    let mut blocks = Vec::new();
    let mut problems = Vec::new();
    let mut in_fence = false;
    let mut fence_lang = String::new();
    let mut fence_start_line = 0usize;
    let mut buffer: Vec<String> = Vec::new();

    for (line_no, line) in record.body.lines().enumerate() {
        let line_no = line_no + 1;
        if let Some(lang) = parse_fence_lang(line) {
            if !in_fence {
                in_fence = true;
                fence_lang = lang.to_lowercase();
                fence_start_line = line_no;
                buffer.clear();
            } else {
                if fence_lang == "mermaid" {
                    let source = buffer.join("\n").trim().to_string();
                    if source.is_empty() {
                        problems.push(Problem::new(
                            "mermaid",
                            record.rel_path.clone(),
                            "empty mermaid block",
                            Some(fence_start_line),
                        ));
                    } else {
                        blocks.push((fence_start_line, source));
                    }
                }
                in_fence = false;
                fence_lang.clear();
                buffer.clear();
            }
            continue;
        }

        if in_fence {
            buffer.push(line.to_string());
        }
    }

    if in_fence {
        problems.push(Problem::new(
            "mermaid",
            record.rel_path.clone(),
            "unclosed code fence",
            Some(fence_start_line),
        ));
    }

    (blocks, problems)
}

fn validate_mermaid(
    root: &Path,
    records: &[DocRecord],
    require_renderer: bool,
) -> (Vec<Problem>, usize) {
    let mut problems = Vec::new();
    let mut count = 0usize;

    for record in records {
        let (blocks, block_problems) = extract_fenced_mermaid_blocks(record);
        problems.extend(block_problems);
        count += blocks.len();
    }

    let mut mmd_files =
        collect_files_with_suffix(&docs_root(root).join("assets"), ".mmd").unwrap_or_default();
    mmd_files.sort();
    for path in mmd_files {
        let rel = rel_posix(root, &path);
        match fs::read_to_string(&path) {
            Ok(text) => {
                if text.trim().is_empty() {
                    problems.push(Problem::new(
                        "mermaid",
                        rel,
                        "empty .mmd diagram file",
                        None,
                    ));
                } else {
                    count += 1;
                }
            }
            Err(err) => problems.push(Problem::new(
                "mermaid",
                rel,
                format!("failed to read mermaid source: {err}"),
                None,
            )),
        }
    }

    if require_renderer {
        eprintln!(
            "warn: `--require-renderer` is deprecated in the Rust-only docs validator; performing structural Mermaid checks only"
        );
    }

    (problems, count)
}

fn validate_openapi(root: &Path, require_validator: bool) -> (Vec<Problem>, usize) {
    let mut problems = Vec::new();
    let openapi_root = docs_root(root).join("reference/openapi");
    let mut specs = Vec::new();
    if let Ok(files) = collect_openapi_specs(&openapi_root) {
        specs = files;
    }

    if require_validator {
        eprintln!(
            "warn: `--require-validator` is deprecated in the Rust-only docs validator; using built-in Rust OpenAPI parse/sanity checks"
        );
    }

    let count = specs.len();
    for spec in specs {
        let rel = rel_posix(root, &spec);
        match spec
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
        {
            Some(ext) if ext == "json" => {
                let text = match fs::read_to_string(&spec) {
                    Ok(text) => text,
                    Err(err) => {
                        problems.push(Problem::new(
                            "openapi",
                            rel,
                            format!("failed to read OpenAPI spec: {err}"),
                            None,
                        ));
                        continue;
                    }
                };
                match serde_json::from_str::<Value>(&text) {
                    Ok(value) => {
                        if !has_openapi_or_swagger_key(&value) {
                            problems.push(Problem::new(
                                "openapi",
                                rel,
                                "missing `openapi` or `swagger` top-level key",
                                None,
                            ));
                        }
                    }
                    Err(err) => problems.push(Problem::new(
                        "openapi",
                        rel,
                        format!("invalid JSON: {err}"),
                        None,
                    )),
                }
            }
            Some(ext) if ext == "yaml" || ext == "yml" => {
                let text = match fs::read_to_string(&spec) {
                    Ok(text) => text,
                    Err(err) => {
                        problems.push(Problem::new(
                            "openapi",
                            rel,
                            format!("failed to read OpenAPI spec: {err}"),
                            None,
                        ));
                        continue;
                    }
                };
                match serde_yaml::from_str::<Value>(&text) {
                    Ok(value) => {
                        if !has_openapi_or_swagger_key(&value) {
                            problems.push(Problem::new(
                                "openapi",
                                rel,
                                "missing `openapi` or `swagger` top-level key",
                                None,
                            ));
                        }
                    }
                    Err(err) => problems.push(Problem::new(
                        "openapi",
                        rel,
                        format!("invalid YAML: {err}"),
                        None,
                    )),
                }
            }
            _ => {}
        }
    }

    (problems, count)
}

fn collect_openapi_specs(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    if !root.exists() {
        return Ok(files);
    }
    collect_openapi_specs_inner(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_openapi_specs_inner(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries: Vec<_> = fs::read_dir(root)
        .map_err(|err| format!("failed to read {}: {err}", root.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| format!("failed to read {}: {err}", root.display()))?;
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_openapi_specs_inner(&path, out)?;
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase());
        if matches!(ext.as_deref(), Some("yaml" | "yml" | "json")) {
            out.push(path);
        }
    }

    Ok(())
}

fn has_openapi_or_swagger_key(value: &Value) -> bool {
    match value {
        Value::Object(map) => map.contains_key("openapi") || map.contains_key("swagger"),
        _ => false,
    }
}

fn run_all(
    root: &Path,
    records: &[DocRecord],
    parse_problems: Vec<Problem>,
    contracts: &Contracts,
    flags: &DocsFlags,
) -> Vec<Problem> {
    let mut problems = parse_problems;
    problems.extend(validate_structure(root, contracts));
    problems.extend(validate_wiki_submodule(root));
    problems.extend(validate_frontmatter(root, records, contracts));
    problems.extend(validate_sop_headings(root, records, contracts));
    problems.extend(validate_links(root, records));
    let (mermaid_problems, _) = validate_mermaid(root, records, flags.require_renderer);
    let (openapi_problems, _) = validate_openapi(root, flags.require_openapi_validator);
    let ui_conformance_problems = validate_ui_conformance(root);
    let storage_boundary_problems = validate_typed_persistence_boundary(root);
    problems.extend(mermaid_problems);
    problems.extend(openapi_problems);
    problems.extend(ui_conformance_problems);
    problems.extend(storage_boundary_problems);
    problems
}

fn write_audit_report(
    root: &Path,
    records: &[DocRecord],
    parse_problems: Vec<Problem>,
    contracts: &Contracts,
    output: &Path,
) -> Result<(), String> {
    let structure_problems = validate_structure(root, contracts);
    let wiki_problems = validate_wiki_submodule(root);
    let frontmatter_problems = validate_frontmatter(root, records, contracts);
    let sop_problems = validate_sop_headings(root, records, contracts);
    let link_problems = validate_links(root, records);
    let (mermaid_problems, mermaid_count) = validate_mermaid(root, records, false);
    let (openapi_problems, openapi_count) = validate_openapi(root, false);
    let ui_conformance_problems = validate_ui_conformance(root);
    let storage_boundary_problems = validate_typed_persistence_boundary(root);

    let (fresh, total_reviewed, stale_docs) = frontmatter_freshness_metrics(records, contracts);
    let fresh_percent = if total_reviewed == 0 {
        0.0
    } else {
        ((fresh as f64 / total_reviewed as f64) * 100.0 * 100.0).round() / 100.0
    };

    let report = json!({
        "generated_at": Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        "document_count": records.len(),
        "markdown_count": records.len(),
        "counts_by_category": counts_by_category(records),
        "fresh_review_percent": fresh_percent,
        "stale_documents": stale_docs,
        "broken_internal_links": link_problems.iter().filter(|p| p.check == "links").count(),
        "mermaid_block_count": mermaid_count,
        "openapi_spec_count": openapi_count,
        "validation_issue_counts": {
            "parse": parse_problems.len(),
            "structure": structure_problems.len(),
            "wiki": wiki_problems.len(),
            "frontmatter": frontmatter_problems.len(),
            "sop": sop_problems.len(),
            "links": link_problems.len(),
            "mermaid": mermaid_problems.len(),
            "openapi": openapi_problems.len(),
            "ui_conformance": ui_conformance_problems.len(),
            "storage_boundary": storage_boundary_problems.len(),
        }
    });

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
    }
    let body = serde_json::to_string_pretty(&report)
        .map_err(|err| format!("failed to serialize report: {err}"))?;
    fs::write(output, format!("{body}\n"))
        .map_err(|err| format!("failed to write {}: {err}", output.display()))?;
    println!("Wrote audit report: {}", output.display());

    let mut all_problems = Vec::new();
    all_problems.extend(parse_problems);
    all_problems.extend(structure_problems);
    all_problems.extend(wiki_problems);
    all_problems.extend(frontmatter_problems);
    all_problems.extend(sop_problems);
    all_problems.extend(link_problems);
    all_problems.extend(mermaid_problems);
    all_problems.extend(openapi_problems);
    all_problems.extend(ui_conformance_problems);
    all_problems.extend(storage_boundary_problems);
    fail_if_problems(all_problems)
}

const FLUENT_OVERRIDES_PATH: &str =
    "crates/site/src/theme_shell/33-theme-fluent-modern-overrides.css";
const TYPED_PERSISTENCE_BOUNDARY_DIRS: &[&str] = &["crates/apps", "crates/desktop_runtime"];
const SHELL_ICON_COMPONENT_FILES: &[&str] = &[
    "crates/desktop_runtime/src/components.rs",
    "crates/desktop_runtime/src/components/display_properties.rs",
    "crates/desktop_runtime/src/components/menus.rs",
    "crates/desktop_runtime/src/components/taskbar.rs",
    "crates/desktop_runtime/src/components/window.rs",
];

fn validate_ui_conformance(root: &Path) -> Vec<Problem> {
    let path = root.join(FLUENT_OVERRIDES_PATH);
    let Ok(text) = fs::read_to_string(&path) else {
        return vec![Problem::new(
            "ui-conformance",
            FLUENT_OVERRIDES_PATH,
            "failed to read Fluent overrides CSS for UI conformance checks",
            None,
        )];
    };

    let mut problems = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            continue;
        }
        if trimmed.starts_with("--") {
            // Token definitions may contain raw literals by design.
            continue;
        }

        if has_disallowed_raw_color_literal(trimmed) {
            problems.push(Problem::new(
                "ui-conformance",
                FLUENT_OVERRIDES_PATH,
                "raw color literal outside token definitions (allowed exception: transparent rgba(..., 0) stops)",
                Some(line_no),
            ));
        }

        if has_disallowed_raw_px_literal(trimmed) {
            problems.push(Problem::new(
                "ui-conformance",
                FLUENT_OVERRIDES_PATH,
                "raw px literal outside token definitions/effect-geometry exceptions",
                Some(line_no),
            ));
        }
    }

    problems.extend(validate_shell_icon_standardization(root));

    problems
}

fn validate_typed_persistence_boundary(root: &Path) -> Vec<Problem> {
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

            for (idx, line) in text.lines().enumerate() {
                if uses_forbidden_envelope_load_call(line) {
                    problems.push(Problem::new(
                        "storage-boundary",
                        rel_path.clone(),
                        "direct `load_app_state_envelope(...)` usage is not allowed in app/runtime crates; use typed load helpers",
                        Some(idx + 1),
                    ));
                }
            }
        }
    }

    problems
}

fn uses_forbidden_envelope_load_call(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return false;
    }
    trimmed.contains("load_app_state_envelope(")
}

fn validate_shell_icon_standardization(root: &Path) -> Vec<Problem> {
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
                problems.push(Problem::new(
                    "ui-conformance",
                    *rel_path,
                    "legacy text glyph icon marker detected in shell component; use semantic `IconName`/`FluentIcon`",
                    Some(line_no),
                ));
            }
        }
    }

    problems
}

fn contains_legacy_shell_icon_text_glyph(line: &str) -> bool {
    // Narrow list of legacy shell icon markers previously used in place of semantic icons.
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

    // Continuation lines for shadow values and gradient stops.
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

fn frontmatter_freshness_metrics(
    records: &[DocRecord],
    contracts: &Contracts,
) -> (usize, usize, Vec<String>) {
    let stale_days =
        env_or_contract_stale_days(contracts).unwrap_or(contracts.stale_review_days as i64);
    let today = current_date().unwrap_or_else(|_| Local::now().date_naive());
    let mut fresh = 0usize;
    let mut total = 0usize;
    let mut stale_docs = Vec::new();

    for record in records {
        let Some(reviewed) = record
            .frontmatter
            .get("last_reviewed")
            .and_then(Value::as_str)
        else {
            continue;
        };
        let Some(parsed) = parse_review_date(reviewed) else {
            continue;
        };
        total += 1;
        if (today - parsed).num_days() <= stale_days {
            fresh += 1;
        } else {
            stale_docs.push(record.rel_path.clone());
        }
    }

    (fresh, total, stale_docs)
}

fn counts_by_category(records: &[DocRecord]) -> BTreeMap<String, usize> {
    let mut counter: BTreeMap<String, usize> = BTreeMap::new();
    for record in records {
        let category = record
            .frontmatter
            .get("category")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        *counter.entry(category).or_default() += 1;
    }
    counter
}

fn fail_if_problems(mut problems: Vec<Problem>) -> Result<(), String> {
    if problems.is_empty() {
        println!("OK");
        return Ok(());
    }
    problems.sort_by(|a, b| {
        (&a.check, &a.path, a.line.unwrap_or(0), &a.message).cmp(&(
            &b.check,
            &b.path,
            b.line.unwrap_or(0),
            &b.message,
        ))
    });
    print_problems(&problems);
    println!("\nFAILED: {} issue(s)", problems.len());
    Err("docs validation failed".to_string())
}

fn print_problems(problems: &[Problem]) {
    for p in problems {
        let loc = match p.line {
            Some(line) => format!("{}:{line}", p.path),
            None => p.path.clone(),
        };
        println!("[{}] {} - {}", p.check, loc, p.message);
    }
}

fn rel_posix(root: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(root).unwrap_or(path);
    rel.components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            Component::CurDir => Some(".".to_string()),
            Component::ParentDir => Some("..".to_string()),
            Component::RootDir | Component::Prefix(_) => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    let mut has_root = false;

    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => {
                has_root = true;
                out.push(Path::new("/"));
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() && !has_root {
                    out.push("..");
                }
            }
            Component::Normal(part) => out.push(part),
        }
    }

    out
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

Basic shell usage.

### Environment Setup

Repo checked out.

### Dependencies

Rust installed.

## Procedure

### Step 1: Do the thing

```md
## Validation
### Prior Knowledge
```

## Validation

Check the thing.

## Next Steps

Continue.
"#;

        let problems = validate_wiki_instructional_page_template("Tutorial-Example.md", text);
        assert!(
            problems.is_empty(),
            "expected no problems, found: {problems:#?}"
        );
    }

    #[test]
    fn wiki_instructional_template_rejects_h2_sequence_drift() {
        let text = r#"# How to: Example

## Outcome
## Entry Criteria
### Prior Knowledge
### Environment Setup
### Dependencies
## Common Mistakes
## Procedure
## Validation
## Next Steps
"#;

        let problems = validate_wiki_instructional_page_template("How-to-Example.md", text);
        assert_eq!(problems.len(), 1, "unexpected problems: {problems:#?}");
        assert!(problems[0]
            .message
            .contains("exact level-2 section sequence"));
    }

    #[test]
    fn wiki_instructional_template_rejects_entry_criteria_subsection_drift() {
        let text = r#"# Tutorial: Example

## Outcome
## Entry Criteria
### Environment Setup
### Prior Knowledge
### Dependencies
## Procedure
## Validation
## Next Steps
"#;

        let problems = validate_wiki_instructional_page_template("Tutorial-Example.md", text);
        assert_eq!(problems.len(), 1, "unexpected problems: {problems:#?}");
        assert!(problems[0]
            .message
            .contains("`## Entry Criteria` must contain exact level-3 subsection sequence"));
    }

    #[test]
    fn storage_boundary_allows_comments_and_flags_calls() {
        assert!(!uses_forbidden_envelope_load_call(
            "// load_app_state_envelope(\"app.example\")"
        ));
        assert!(uses_forbidden_envelope_load_call(
            "let _ = platform_storage::load_app_state_envelope(\"app.example\").await;"
        ));
    }
}
