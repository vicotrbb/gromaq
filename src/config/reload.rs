//! Deterministic config-file reload tracking.

use std::path::{Path, PathBuf};

use crate::error::Result;

use super::{GromaqConfig, read_config_contents};

/// Result of checking a configuration file for reloadable changes.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigReload {
    /// Whether the underlying file contents changed and parsed successfully.
    pub changed: bool,
    /// The current validated configuration after the check.
    pub config: GromaqConfig,
}

/// Deterministic polling helper for validated TOML config reloads.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigFileReloader {
    path: PathBuf,
    contents: String,
    config: GromaqConfig,
}

impl ConfigFileReloader {
    /// Load an initial config file and retain its validated contents for future reload checks.
    pub fn from_file(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let contents = read_config_contents(&path)?;
        let config = GromaqConfig::from_toml_str(&contents)?;
        Ok(Self {
            path,
            contents,
            config,
        })
    }

    /// Path watched by this reloader.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Last successfully validated configuration.
    pub fn current(&self) -> &GromaqConfig {
        &self.config
    }

    /// Reload the file when contents changed.
    ///
    /// Invalid changed contents return an error and leave the previous validated config intact.
    pub fn reload_if_changed(&mut self) -> Result<ConfigReload> {
        let contents = read_config_contents(&self.path)?;
        if contents == self.contents {
            return Ok(ConfigReload {
                changed: false,
                config: self.config.clone(),
            });
        }
        let config = GromaqConfig::from_toml_str(&contents)?;
        self.contents = contents;
        self.config = config.clone();
        Ok(ConfigReload {
            changed: true,
            config,
        })
    }
}
