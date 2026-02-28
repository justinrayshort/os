use super::*;

pub(crate) fn run_all(
    root: &Path,
    records: &[DocRecord],
    parse_problems: Vec<Problem>,
    contracts: &Contracts,
    flags: &DocsFlags,
) -> Vec<Problem> {
    let mut problems = parse_problems;
    problems.extend(structure::validate_structure(root, contracts));
    problems.extend(wiki::validate_wiki_submodule(root));
    problems.extend(frontmatter::validate_frontmatter(root, records, contracts));
    problems.extend(sop::validate_sop_headings(root, records, contracts));
    problems.extend(links::validate_links(root, records));
    let (mermaid_problems, _) = mermaid::validate_mermaid(root, records, flags.require_renderer);
    let (openapi_problems, _) = openapi::validate_openapi(root, flags.require_openapi_validator);
    let ui_conformance_problems = ui_conformance::validate_ui_conformance(root);
    let storage_boundary_problems = storage_boundary::validate_typed_persistence_boundary(root);
    let app_contract_problems = app_contract::validate_app_contracts(root);
    problems.extend(mermaid_problems);
    problems.extend(openapi_problems);
    problems.extend(ui_conformance_problems);
    problems.extend(storage_boundary_problems);
    problems.extend(app_contract_problems);
    problems
}

pub(crate) fn write_audit_report(
    root: &Path,
    records: &[DocRecord],
    parse_problems: Vec<Problem>,
    contracts: &Contracts,
    output: &Path,
) -> Result<(), String> {
    let structure_problems = structure::validate_structure(root, contracts);
    let wiki_problems = wiki::validate_wiki_submodule(root);
    let frontmatter_problems = frontmatter::validate_frontmatter(root, records, contracts);
    let sop_problems = sop::validate_sop_headings(root, records, contracts);
    let link_problems = links::validate_links(root, records);
    let (mermaid_problems, mermaid_count) = mermaid::validate_mermaid(root, records, false);
    let (openapi_problems, openapi_count) = openapi::validate_openapi(root, false);
    let ui_conformance_problems = ui_conformance::validate_ui_conformance(root);
    let storage_boundary_problems = storage_boundary::validate_typed_persistence_boundary(root);
    let app_contract_problems = app_contract::validate_app_contracts(root);

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
            "app_contract": app_contract_problems.len(),
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
    all_problems.extend(app_contract_problems);
    fail_if_problems(all_problems)
}

fn frontmatter_freshness_metrics(
    records: &[DocRecord],
    contracts: &Contracts,
) -> (usize, usize, Vec<String>) {
    let stale_days = frontmatter::env_or_contract_stale_days(contracts)
        .unwrap_or(contracts.stale_review_days as i64);
    let today = frontmatter::current_date().unwrap_or_else(|_| Local::now().date_naive());
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
        let Some(parsed) = frontmatter::parse_review_date(reviewed) else {
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

pub(crate) fn fail_if_problems(mut problems: Vec<Problem>) -> Result<(), String> {
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
