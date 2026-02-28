use super::*;

pub(crate) fn validate_links(root: &Path, records: &[DocRecord]) -> Vec<Problem> {
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
