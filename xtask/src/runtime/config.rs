//! Typed configuration loading helpers.

use crate::runtime::error::{XtaskError, XtaskResult};
use serde::de::DeserializeOwned;
use std::fs;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

/// Generic TOML-backed config loader.
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
    pub fn load(&self) -> XtaskResult<T> {
        let body = fs::read_to_string(&self.path).map_err(|err| {
            XtaskError::config(format!("failed to read {}: {err}", self.path.display()))
        })?;
        toml::from_str(&body).map_err(|err| {
            XtaskError::config(format!("failed to parse {}: {err}", self.path.display()))
        })
    }

    /// Config path on disk.
    pub fn path(&self) -> &Path {
        &self.path
    }
}
