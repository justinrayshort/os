use chrono::{Local, NaiveDate, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};

mod app_contract;
mod audit;
mod frontmatter;
mod links;
mod mermaid;
mod openapi;
mod sop;
mod storage_boundary;
mod structure;
mod ui_conformance;
mod wiki;

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
pub(crate) struct Problem {
    check: String,
    path: String,
    message: String,
    line: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct UiInventoryEntry {
    entrypoint_type: String,
    owner_layer: String,
    selector_or_token: String,
    file: String,
    line: usize,
    classification: String,
    recommended_replacement: String,
}

impl Problem {
    pub(crate) fn new(
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
pub(crate) struct LinkRef {
    target: String,
    line: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct DocRecord {
    path: PathBuf,
    rel_path: String,
    frontmatter: Map<String, Value>,
    body: String,
    headings: Vec<String>,
    anchors: HashSet<String>,
    links: Vec<LinkRef>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Contracts {
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
pub(crate) struct DocsFlags {
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
    UiInventory,
    StorageBoundary,
    AppContract,
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
           ui-inventory --output <path>      Write styling entrypoint inventory JSON\n\
           storage-boundary                  Enforce typed app-state envelope boundary in app/runtime crates\n\
           app-contract                      Validate app manifest and contract conventions\n\
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
        DocsCommand::Structure => {
            audit::fail_if_problems(structure::validate_structure(root, &contracts))
        }
        DocsCommand::Wiki => audit::fail_if_problems(wiki::validate_wiki_submodule(root)),
        DocsCommand::Frontmatter => {
            let mut problems = parse_problems;
            problems.extend(frontmatter::validate_frontmatter(
                root, &records, &contracts,
            ));
            audit::fail_if_problems(problems)
        }
        DocsCommand::Sop => {
            audit::fail_if_problems(sop::validate_sop_headings(root, &records, &contracts))
        }
        DocsCommand::Links => audit::fail_if_problems(links::validate_links(root, &records)),
        DocsCommand::Mermaid => {
            let (problems, count) =
                mermaid::validate_mermaid(root, &records, flags.require_renderer);
            println!("Mermaid sources checked: {count}");
            audit::fail_if_problems(problems)
        }
        DocsCommand::OpenApi => {
            let (problems, count) =
                openapi::validate_openapi(root, flags.require_openapi_validator);
            println!("OpenAPI specs checked: {count}");
            audit::fail_if_problems(problems)
        }
        DocsCommand::UiConformance => {
            audit::fail_if_problems(ui_conformance::validate_ui_conformance(root))
        }
        DocsCommand::UiInventory => {
            let output = audit_output.ok_or_else(|| "missing `--output <path>`".to_string())?;
            ui_conformance::write_ui_inventory(root, &output)
        }
        DocsCommand::StorageBoundary => {
            audit::fail_if_problems(storage_boundary::validate_typed_persistence_boundary(root))
        }
        DocsCommand::AppContract => {
            audit::fail_if_problems(app_contract::validate_app_contracts(root))
        }
        DocsCommand::All => {
            let problems = audit::run_all(root, &records, parse_problems, &contracts, &flags);
            audit::fail_if_problems(problems)
        }
        DocsCommand::AuditReport => {
            let output = audit_output.ok_or_else(|| "missing `--output <path>`".to_string())?;
            audit::write_audit_report(root, &records, parse_problems, &contracts, &output)
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
        "ui-inventory" => DocsCommand::UiInventory,
        "storage-boundary" => DocsCommand::StorageBoundary,
        "app-contract" => DocsCommand::AppContract,
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
            "--require-validator" | "--require-openapi-validator" => {
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
        | DocsCommand::StorageBoundary
        | DocsCommand::AppContract => {
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
        DocsCommand::UiInventory => {
            if flags.require_renderer || flags.require_openapi_validator {
                return Err("`ui-inventory` does not accept validator flags".to_string());
            }
            if output.is_none() {
                return Err("`ui-inventory` requires `--output <path>`".to_string());
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

pub(crate) fn docs_root(root: &Path) -> PathBuf {
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

pub(crate) fn collect_files_with_suffix(root: &Path, suffix: &str) -> Result<Vec<PathBuf>, String> {
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
                Some(Value::Array(items)) => items.push(parse_scalar(item_text)),
                _ => errors.push(format!(
                    "frontmatter line {line_num}: list item without list key"
                )),
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
            data.insert(key.to_string(), Value::Array(parse_inline_list(value)));
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

pub(crate) fn strip_quotes(value: &str) -> &str {
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
    let inner = value[1..value.len() - 1].trim();
    if inner.is_empty() {
        return Vec::new();
    }
    let mut items = Vec::new();
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

pub(crate) fn parse_fence_lang(line: &str) -> Option<String> {
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

pub(crate) fn parse_markdown_heading_parts(line: &str) -> Option<(usize, &str)> {
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

pub(crate) fn normalize_heading(value: &str) -> String {
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

pub(crate) fn rel_posix(root: &Path, path: &Path) -> String {
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

pub(crate) fn normalize_path(path: &Path) -> PathBuf {
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
