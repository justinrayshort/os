use super::*;

pub(crate) fn validate_sop_headings(
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
                Some(found_rel) => pos += found_rel + 1,
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
