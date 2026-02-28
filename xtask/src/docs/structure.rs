use super::*;

pub(crate) fn validate_structure(root: &Path, contracts: &Contracts) -> Vec<Problem> {
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
