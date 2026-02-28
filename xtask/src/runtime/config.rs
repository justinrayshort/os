//! Typed configuration loading helpers.

use crate::runtime::error::{XtaskError, XtaskResult};
use serde::de::DeserializeOwned;
use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

/// Generic TOML-backed config loader.
///
/// `ConfigLoader<T>` handles only filesystem access and TOML deserialization. Consuming command
/// domains are still responsible for semantic validation after the typed value is loaded.
///
/// Typical usage:
///
/// ```rust
/// # use serde::Deserialize;
/// # use std::path::Path;
/// # use xtask::runtime::config::ConfigLoader;
/// #[derive(Deserialize)]
/// struct ExampleConfig {
///     enabled: bool,
/// }
///
/// let loader = ConfigLoader::<ExampleConfig>::new(Path::new("/workspace"), "tools/example.toml");
/// let _ = loader.path();
/// ```
#[derive(Clone, Debug)]
pub struct ConfigLoader<T> {
    path: PathBuf,
    _marker: PhantomData<T>,
}

impl<T> ConfigLoader<T>
where
    T: DeserializeOwned,
{
    /// Create a loader for the given workspace-relative path.
    pub fn new(root: &Path, relative_path: &str) -> Self {
        Self {
            path: root.join(relative_path),
            _marker: PhantomData,
        }
    }

    /// Load and deserialize the configuration file.
    ///
    /// Missing files, unreadable files, and TOML parse failures are all surfaced as
    /// [`XtaskErrorCategory::Config`](crate::runtime::error::XtaskErrorCategory::Config).
    pub fn load(&self) -> XtaskResult<T> {
        let body = fs::read_to_string(&self.path).map_err(|err| {
            XtaskError::config(format!("failed to read {}: {err}", self.path.display()))
        })?;
        toml::from_str(&body).map_err(|err| {
            XtaskError::config(format!("failed to parse {}: {err}", self.path.display()))
        })
    }

    /// Return the config path on disk.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::error::XtaskErrorCategory;
    use serde::Deserialize;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct ExampleConfig {
        value: String,
        count: u32,
    }

    fn unique_test_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "xtask-config-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }

    #[test]
    fn load_reads_toml_config_from_workspace_relative_path() {
        let root = unique_test_root();
        let config_dir = root.join("tools/automation");
        fs::create_dir_all(&config_dir).expect("create config dir");
        fs::write(
            config_dir.join("example.toml"),
            "value = \"ok\"\ncount = 7\n",
        )
        .expect("write config");

        let loader = ConfigLoader::<ExampleConfig>::new(&root, "tools/automation/example.toml");
        let loaded = loader.load().expect("load config");
        assert_eq!(
            loaded,
            ExampleConfig {
                value: "ok".into(),
                count: 7,
            }
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_reports_missing_file_as_config_error() {
        let root = unique_test_root();
        fs::create_dir_all(&root).expect("create temp root");

        let loader = ConfigLoader::<ExampleConfig>::new(&root, "tools/automation/missing.toml");
        let err = loader.load().expect_err("missing config should fail");
        assert_eq!(err.category, XtaskErrorCategory::Config);
        assert!(err.to_string().contains("missing.toml"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_reports_invalid_toml_as_config_error() {
        let root = unique_test_root();
        let config_dir = root.join("tools/automation");
        fs::create_dir_all(&config_dir).expect("create config dir");
        fs::write(config_dir.join("broken.toml"), "value = [").expect("write broken config");

        let loader = ConfigLoader::<ExampleConfig>::new(&root, "tools/automation/broken.toml");
        let err = loader.load().expect_err("invalid config should fail");
        assert_eq!(err.category, XtaskErrorCategory::Config);
        assert!(err.to_string().contains("broken.toml"));

        let _ = fs::remove_dir_all(root);
    }
}
