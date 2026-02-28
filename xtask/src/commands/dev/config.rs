use super::DEV_SERVER_CONFIG_FILE;
use crate::runtime::config::ConfigLoader;
use crate::runtime::context::CommandContext;
use crate::runtime::error::{XtaskError, XtaskResult};
use serde::Deserialize;
use std::time::Duration;

#[derive(Clone, Debug, Deserialize)]
struct DevServerConfigFile {
    dev_server: DevServerConfig,
}

/// Typed development server configuration loaded from `tools/automation/dev_server.toml`.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct DevServerConfig {
    pub(crate) dir: String,
    pub(crate) state_file: String,
    pub(crate) log_file: String,
    pub(crate) default_host: String,
    pub(crate) default_port: u16,
    pub(crate) start_poll_secs: u64,
    pub(crate) stop_timeout_secs: u64,
}

impl DevServerConfig {
    fn validate(self) -> XtaskResult<Self> {
        if self.dir.is_empty() || self.state_file.is_empty() || self.log_file.is_empty() {
            return Err(XtaskError::config(
                "dev_server config paths must not be empty",
            ));
        }
        if self.start_poll_secs == 0 || self.stop_timeout_secs == 0 {
            return Err(XtaskError::config(
                "dev_server timeout values must be greater than zero",
            ));
        }
        Ok(self)
    }

    pub(crate) fn start_poll(&self) -> Duration {
        Duration::from_secs(self.start_poll_secs)
    }

    pub(crate) fn stop_timeout(&self) -> Duration {
        Duration::from_secs(self.stop_timeout_secs)
    }
}

pub(crate) fn load_dev_server_config(ctx: &CommandContext) -> XtaskResult<DevServerConfig> {
    let loader = ConfigLoader::<DevServerConfigFile>::new(ctx.root(), DEV_SERVER_CONFIG_FILE);
    loader.load()?.dev_server.validate()
}
