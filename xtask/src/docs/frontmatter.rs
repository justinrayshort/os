use super::*;

pub(crate) fn validate_frontmatter(
    root: &Path,
    records: &[DocRecord],
    contracts: &Contracts,
) -> Vec<Problem> {
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

pub(crate) fn parse_review_date(value: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()
}

pub(crate) fn current_date() -> Result<NaiveDate, String> {
    if let Ok(override_date) = env::var("DOCS_TODAY") {
        return parse_review_date(&override_date).ok_or_else(|| {
            format!("invalid DOCS_TODAY date `{override_date}` (expected YYYY-MM-DD)")
        });
    }
    Ok(Local::now().date_naive())
}

pub(crate) fn env_or_contract_stale_days(contracts: &Contracts) -> Result<i64, String> {
    if let Ok(value) = env::var("DOCS_STALE_REVIEW_DAYS") {
        let parsed = value
            .parse::<i64>()
            .map_err(|_| format!("invalid DOCS_STALE_REVIEW_DAYS `{value}`"))?;
        return Ok(parsed);
    }
    Ok(contracts.stale_review_days as i64)
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
