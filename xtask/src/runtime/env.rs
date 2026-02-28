//! Environment normalization helpers.

use std::env;
use std::process::Command;

/// Shared environment helper utilities.
#[derive(Clone, Copy, Debug, Default)]
pub struct EnvHelper;

impl EnvHelper {
    /// Normalize `NO_COLOR` values that downstream CLIs parse strictly.
    pub fn normalized_no_color_value(raw: Option<&str>) -> Option<&'static str> {
        match raw {
            Some("1") => Some("true"),
            _ => None,
        }
    }

    /// Apply `NO_COLOR` normalization to a command if needed.
    pub fn apply_no_color_override(&self, cmd: &mut Command) {
        if let Some(value) = Self::normalized_no_color_value(env::var("NO_COLOR").ok().as_deref()) {
            cmd.env("NO_COLOR", value);
        }
    }
}
