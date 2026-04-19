//! Analytics queries on the snapshots database.

use crate::error::Result;
use crate::model::UsageSnapshot;
use crate::storage::Database;
use serde::Serialize;

/// How many % per hour the session is consuming (averaged over last N snapshots
/// within the current session window).
#[derive(Debug, Clone, Serialize)]
pub struct VelocityStats {
    pub pct_per_hour: f32,
    pub sample_count: u32,
    /// Predicted time (epoch ms UTC) until session_pct hits 100%, or None if
    /// velocity ≤ 0.
    pub eta_to_100_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HeatmapCell {
    /// 0 = Sunday, 6 = Saturday.
    pub day_of_week: u8,
    /// 0..=23 (local time).
    pub hour: u8,
    pub avg_session_pct: f32,
    pub samples: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub total_snapshots: i64,
    pub last_snapshot: Option<UsageSnapshot>,
    pub velocity: Option<VelocityStats>,
    pub recent_series: Vec<UsageSnapshot>,
    pub heatmap: Vec<HeatmapCell>,
}

pub fn summary(db: &Database, window_hours: i64) -> Result<Summary> {
    let now_ms = jiff::Timestamp::now().as_millisecond();
    let since_ms = now_ms - window_hours * 3600 * 1000;
    let recent = db.snapshots_since(since_ms)?;
    let total = db.count()?;
    let last = db.last_snapshot()?;
    let velocity = compute_velocity(&recent);
    let heatmap = compute_heatmap(db, 30)?;
    Ok(Summary {
        total_snapshots: total,
        last_snapshot: last,
        velocity,
        recent_series: recent,
        heatmap,
    })
}

/// Velocity = (last_pct - first_pct) / hours_span, scoped to the current session.
/// "Current session" = contiguous range with matching session_resets_at_ms.
pub fn compute_velocity(series: &[UsageSnapshot]) -> Option<VelocityStats> {
    if series.len() < 2 {
        return None;
    }
    let latest = series.last()?;
    let reset = latest.session_resets_at_ms?;
    // Filter to same session.
    let same: Vec<&UsageSnapshot> = series
        .iter()
        .filter(|s| s.session_resets_at_ms == Some(reset))
        .collect();
    if same.len() < 2 {
        return None;
    }
    let first = same.first()?;
    let last = same.last()?;
    let delta_pct = last.session_pct - first.session_pct;
    let delta_hours = (last.timestamp_ms - first.timestamp_ms) as f32 / 3_600_000.0;
    if delta_hours < 0.05 {
        return None; // less than 3 min of data — unreliable
    }
    let velocity = delta_pct / delta_hours;
    let eta_to_100_ms = if velocity > 0.0 {
        let remaining_pct = 100.0 - last.session_pct;
        let hours_to_100 = remaining_pct / velocity;
        Some(last.timestamp_ms + (hours_to_100 * 3_600_000.0) as i64)
    } else {
        None
    };
    Some(VelocityStats {
        pct_per_hour: velocity,
        sample_count: same.len() as u32,
        eta_to_100_ms,
    })
}

pub fn compute_heatmap(db: &Database, days: i64) -> Result<Vec<HeatmapCell>> {
    let since_ms = jiff::Timestamp::now().as_millisecond() - days * 86_400 * 1000;
    let rows = db.snapshots_since(since_ms)?;

    let mut buckets: std::collections::HashMap<(u8, u8), (f64, u32)> = Default::default();
    for s in &rows {
        let ts = jiff::Timestamp::from_millisecond(s.timestamp_ms)
            .unwrap_or(jiff::Timestamp::UNIX_EPOCH);
        // Use local zone for human readability.
        let zoned = ts.to_zoned(jiff::tz::TimeZone::system());
        let dow = zoned.weekday().to_sunday_zero_offset() as u8; // 0=Sunday
        let hour = zoned.hour() as u8;
        let entry = buckets.entry((dow, hour)).or_insert((0.0, 0));
        entry.0 += s.session_pct as f64;
        entry.1 += 1;
    }

    let mut out: Vec<HeatmapCell> = buckets
        .into_iter()
        .map(|((dow, hour), (sum, count))| HeatmapCell {
            day_of_week: dow,
            hour,
            avg_session_pct: if count > 0 {
                (sum / count as f64) as f32
            } else {
                0.0
            },
            samples: count,
        })
        .collect();
    out.sort_by_key(|c| (c.day_of_week, c.hour));
    Ok(out)
}
