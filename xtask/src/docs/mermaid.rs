use super::*;

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

pub(crate) fn validate_mermaid(
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
