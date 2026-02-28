use super::*;

pub(crate) fn validate_openapi(root: &Path, require_validator: bool) -> (Vec<Problem>, usize) {
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
