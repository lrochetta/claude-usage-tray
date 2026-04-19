use crate::error::{CoreError, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const QUALIFIER: &str = "sh";
const ORGANIZATION: &str = "laurent";
const APPLICATION: &str = "claude-usage-tray";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Minimum interval between OAuth API calls, in seconds.
    /// Rate limit on the endpoint is severe; do not go below 300.
    #[serde(default = "default_api_poll_secs")]
    pub api_poll_secs: u64,

    /// Fine-grained polling (local JSONL scan only) in seconds.
    /// Cheap — just reads local files to estimate token consumption between API calls.
    #[serde(default = "default_local_poll_secs")]
    pub local_poll_secs: u64,

    /// How many days of raw snapshots to keep in SQLite.
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,

    /// Start with Windows on login.
    #[serde(default)]
    pub autostart: bool,

    /// Notification threshold — trigger a toast when session % crosses this.
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold_pct: f32,

    /// Override for ~/.claude/.credentials.json path (advanced).
    #[serde(default)]
    pub credentials_path_override: Option<PathBuf>,
}

fn default_api_poll_secs() -> u64 {
    300 // 5 min
}
fn default_local_poll_secs() -> u64 {
    60 // 1 min
}
fn default_retention_days() -> u32 {
    90
}
fn default_alert_threshold() -> f32 {
    90.0
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_poll_secs: default_api_poll_secs(),
            local_poll_secs: default_local_poll_secs(),
            retention_days: default_retention_days(),
            autostart: false,
            alert_threshold_pct: default_alert_threshold(),
            credentials_path_override: None,
        }
    }
}

impl Config {
    /// Load config from disk, or return default if missing.
    pub fn load_or_default() -> Result<Self> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(&path)?;
        let cfg: Self = toml::from_str(&text)?;
        Ok(cfg)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)?;
        std::fs::write(&path, text)?;
        Ok(())
    }

    pub fn config_path() -> Result<PathBuf> {
        let dirs = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION).ok_or_else(|| {
            CoreError::Config("could not resolve project config directory".into())
        })?;
        Ok(dirs.config_dir().join("config.toml"))
    }

    pub fn data_dir() -> Result<PathBuf> {
        let dirs = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)
            .ok_or_else(|| CoreError::Config("could not resolve project data directory".into()))?;
        let p = dirs.data_dir().to_path_buf();
        std::fs::create_dir_all(&p)?;
        Ok(p)
    }

    pub fn database_path() -> Result<PathBuf> {
        Ok(Self::data_dir()?.join("usage.sqlite"))
    }

    /// Path to the Claude Code CLI credentials file (~/.claude/.credentials.json on unix,
    /// %USERPROFILE%\.claude\.credentials.json on Windows).
    pub fn credentials_path(&self) -> Result<PathBuf> {
        if let Some(p) = &self.credentials_path_override {
            return Ok(p.clone());
        }
        let home = directories::UserDirs::new()
            .map(|u| u.home_dir().to_path_buf())
            .ok_or_else(|| CoreError::Config("could not resolve home directory".into()))?;
        Ok(home.join(".claude").join(".credentials.json"))
    }

    /// Enforce polling floor to protect against rate limit.
    pub fn effective_api_poll_secs(&self) -> u64 {
        self.api_poll_secs.max(300)
    }

    pub fn effective_local_poll_secs(&self) -> u64 {
        self.local_poll_secs.max(30)
    }
}

/// Helper for tests / explicit paths.
pub fn config_file_at(path: &Path) -> Result<Config> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let text = std::fs::read_to_string(path)?;
    Ok(toml::from_str(&text)?)
}
