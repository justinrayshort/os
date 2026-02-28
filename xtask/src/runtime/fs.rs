//! Shared filesystem helpers for xtask workflows.

use crate::runtime::error::{XtaskError, XtaskResult};
use std::fs;
use std::path::Path;

/// Read the last `max_lines` lines from a text file.
pub fn read_file_tail(path: &Path, max_lines: usize) -> XtaskResult<String> {
    let contents = fs::read_to_string(path)
        .map_err(|err| XtaskError::io(format!("failed to read {}: {err}", path.display())))?;
    let lines: Vec<&str> = contents.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    Ok(lines[start..].join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_file() -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "xtask-fs-test-{}.txt",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    #[test]
    fn read_file_tail_returns_last_lines() {
        let path = unique_temp_file();
        fs::write(&path, "a\nb\nc\nd\n").expect("write temp file");
        let tail = read_file_tail(&path, 2).expect("read tail");
        assert_eq!(tail, "c\nd");
        let _ = fs::remove_file(path);
    }
}
