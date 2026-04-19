//! claude-usage-tray-core
//!
//! Core library: fetch, store, and analyze Claude Code usage metrics.
//! UI-agnostic — consumed by the public tray binary and the internal variant.

pub mod analytics;
pub mod config;
pub mod error;
pub mod fetch;
pub mod model;
pub mod report;
pub mod storage;

pub use config::Config;
pub use error::{CoreError, Result};
pub use fetch::{fetch_usage, fetch_usage_from_credentials, fetch_usage_with_config};
pub use model::{ThresholdColor, UsageSnapshot};
pub use storage::Database;
