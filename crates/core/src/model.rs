use serde::{Deserialize, Serialize};

/// Single observation of Claude Code usage limits at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageSnapshot {
    /// Epoch milliseconds UTC.
    pub timestamp_ms: i64,
    /// 0.0..=100.0 — current 5h session usage percentage.
    pub session_pct: f32,
    /// Epoch ms UTC when the current 5h session resets, if known.
    pub session_resets_at_ms: Option<i64>,
    /// 0.0..=100.0 — weekly "all models" bucket.
    pub weekly_all_pct: f32,
    /// Optional — weekly Sonnet-specific bucket.
    pub weekly_sonnet_pct: Option<f32>,
    /// Optional — weekly Opus / Design bucket.
    pub weekly_design_pct: Option<f32>,
    /// Optional — epoch ms UTC when the weekly window resets.
    pub weekly_resets_at_ms: Option<i64>,
    /// Optional — daily routine runs used.
    pub daily_routines_used: Option<u32>,
    /// Optional — daily routine runs limit.
    pub daily_routines_limit: Option<u32>,
    /// Raw JSON payload we got back, for forward-compatibility / debugging.
    pub raw_payload: Option<String>,
}

impl UsageSnapshot {
    /// Map current session_pct to a threshold color.
    pub fn color(&self) -> ThresholdColor {
        ThresholdColor::from_pct(self.session_pct)
    }

    /// Human-friendly tooltip for the tray icon.
    pub fn tooltip(&self) -> String {
        let reset_str = self
            .session_resets_at_ms
            .map(format_reset_countdown)
            .unwrap_or_else(|| "unknown".to_string());
        format!(
            "Claude Code\nSession: {:.0}% — resets in {}\nWeekly (all): {:.0}%",
            self.session_pct, reset_str, self.weekly_all_pct
        )
    }
}

/// Icon color buckets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThresholdColor {
    Green,
    Yellow,
    Orange,
    Red,
}

impl ThresholdColor {
    pub fn from_pct(pct: f32) -> Self {
        if pct < 50.0 {
            ThresholdColor::Green
        } else if pct < 80.0 {
            ThresholdColor::Yellow
        } else if pct < 95.0 {
            ThresholdColor::Orange
        } else {
            ThresholdColor::Red
        }
    }

    /// RGBA bytes for a filled circle on transparent background.
    pub fn rgba(self) -> [u8; 4] {
        match self {
            ThresholdColor::Green => [0x22, 0xC5, 0x5E, 0xFF],
            ThresholdColor::Yellow => [0xEA, 0xB3, 0x08, 0xFF],
            ThresholdColor::Orange => [0xF9, 0x73, 0x16, 0xFF],
            ThresholdColor::Red => [0xDC, 0x26, 0x26, 0xFF],
        }
    }
}

/// Format a reset timestamp as "Xh YYm" countdown from now.
pub fn format_reset_countdown(reset_ms: i64) -> String {
    let now_ms = jiff::Timestamp::now().as_millisecond();
    let delta_ms = (reset_ms - now_ms).max(0);
    let secs = delta_ms / 1000;
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    if hours > 0 {
        format!("{}h {:02}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}
