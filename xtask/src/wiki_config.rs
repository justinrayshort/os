//! Shared GitHub Wiki configuration and checkout resolution.

use crate::runtime::error::{XtaskError, XtaskResult};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const WIKI_CONFIG_PATH: &str = "tools/docs/wiki.toml";

/// Versioned configuration describing how the workspace integrates with the external wiki repo.
#[derive(Clone, Debug, Deserialize)]
pub struct WikiConfig {
    /// Git remote URL for the external wiki repository.
    pub remote_url: String,
    /// Expected default branch for the wiki repository.
    pub default_branch: String,
    /// Recommended parent directory for the external wiki checkout.
    pub recommended_checkout_parent: String,
    /// Recommended checkout directory name for the external wiki checkout.
    pub recommended_checkout_name: String,
}

/// Resolved external wiki checkout details.
#[derive(Clone, Debug)]
pub struct WikiCheckout {
    /// Versioned config loaded from `tools/docs/wiki.toml`.
    pub config: WikiConfig,
    /// Resolved checkout path.
    pub path: PathBuf,
    /// Human-readable description of how the path was chosen.
    pub source: &'static str,
}

/// Load the versioned wiki config from the workspace.
pub fn load_wiki_config(root: &Path) -> XtaskResult<WikiConfig> {
    let path = root.join(WIKI_CONFIG_PATH);
    let text = fs::read_to_string(&path).map_err(|err| {
        XtaskError::io(format!("failed to read {}: {err}", path.display()))
            .with_operation("load wiki config")
            .with_path(&path)
    })?;
    toml::from_str(&text).map_err(|err| {
        XtaskError::config(format!("failed to parse {}: {err}", path.display()))
            .with_operation("parse wiki config")
            .with_path(&path)
    })
}

/// Resolve the expected local wiki checkout path from env/config defaults.
pub fn resolve_wiki_checkout(root: &Path) -> XtaskResult<WikiCheckout> {
    let config = load_wiki_config(root)?;

    if let Ok(raw) = env::var("OS_WIKI_PATH") {
        if raw.trim().is_empty() {
            return Err(XtaskError::validation(
                "OS_WIKI_PATH is set but empty; expected a filesystem path",
            ));
        }
        let candidate = PathBuf::from(&raw);
        let path = if candidate.is_absolute() {
            candidate
        } else {
            root.join(candidate)
        };
        return Ok(WikiCheckout {
            config,
            path,
            source: "OS_WIKI_PATH",
        });
    }

    let parent = PathBuf::from(&config.recommended_checkout_parent);
    let path = if parent.is_absolute() {
        parent.join(&config.recommended_checkout_name)
    } else {
        root.join(parent).join(&config.recommended_checkout_name)
    };

    Ok(WikiCheckout {
        config,
        path,
        source: "tools/docs/wiki.toml",
    })
}

/// Return a CLI-friendly recommended checkout path string for messages and docs.
pub fn recommended_checkout_display(root: &Path) -> XtaskResult<String> {
    let checkout = resolve_wiki_checkout(root)?;
    Ok(checkout.path.display().to_string())
}
