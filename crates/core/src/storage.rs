use crate::error::Result;
use crate::model::UsageSnapshot;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

const SCHEMA_VERSION: i64 = 1;

const SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts_ms INTEGER NOT NULL,
    session_pct REAL NOT NULL,
    session_resets_at_ms INTEGER,
    weekly_all_pct REAL NOT NULL,
    weekly_sonnet_pct REAL,
    weekly_design_pct REAL,
    weekly_resets_at_ms INTEGER,
    daily_routines_used INTEGER,
    daily_routines_limit INTEGER,
    raw_payload TEXT
);

CREATE INDEX IF NOT EXISTS idx_snapshots_ts ON snapshots(ts_ms);
CREATE INDEX IF NOT EXISTS idx_snapshots_session_reset ON snapshots(session_resets_at_ms);

CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
"#;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path.as_ref())?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.execute_batch(SCHEMA_SQL)?;

        conn.execute(
            "INSERT OR REPLACE INTO meta(key, value) VALUES(?, ?)",
            params!["schema_version", SCHEMA_VERSION.to_string()],
        )?;

        Ok(Self { conn })
    }

    /// Open a read-only connection to an existing database (safe to open from a
    /// secondary thread without risk of schema mutation).
    pub fn open_readonly<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn =
            Connection::open_with_flags(path.as_ref(), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        Ok(Self { conn })
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn insert(&self, snap: &UsageSnapshot) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO snapshots (
                ts_ms, session_pct, session_resets_at_ms,
                weekly_all_pct, weekly_sonnet_pct, weekly_design_pct, weekly_resets_at_ms,
                daily_routines_used, daily_routines_limit, raw_payload
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                snap.timestamp_ms,
                snap.session_pct,
                snap.session_resets_at_ms,
                snap.weekly_all_pct,
                snap.weekly_sonnet_pct,
                snap.weekly_design_pct,
                snap.weekly_resets_at_ms,
                snap.daily_routines_used,
                snap.daily_routines_limit,
                snap.raw_payload,
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn last_snapshot(&self) -> Result<Option<UsageSnapshot>> {
        let row = self
            .conn
            .query_row(
                "SELECT ts_ms, session_pct, session_resets_at_ms,
                        weekly_all_pct, weekly_sonnet_pct, weekly_design_pct, weekly_resets_at_ms,
                        daily_routines_used, daily_routines_limit, raw_payload
                 FROM snapshots ORDER BY ts_ms DESC LIMIT 1",
                [],
                row_to_snapshot,
            )
            .optional()?;
        Ok(row)
    }

    pub fn snapshots_since(&self, since_ms: i64) -> Result<Vec<UsageSnapshot>> {
        let mut stmt = self.conn.prepare(
            "SELECT ts_ms, session_pct, session_resets_at_ms,
                    weekly_all_pct, weekly_sonnet_pct, weekly_design_pct, weekly_resets_at_ms,
                    daily_routines_used, daily_routines_limit, raw_payload
             FROM snapshots WHERE ts_ms >= ? ORDER BY ts_ms ASC",
        )?;
        let rows = stmt
            .query_map(params![since_ms], row_to_snapshot)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn purge_older_than(&self, cutoff_ms: i64) -> Result<usize> {
        let n = self
            .conn
            .execute("DELETE FROM snapshots WHERE ts_ms < ?", params![cutoff_ms])?;
        Ok(n)
    }

    pub fn count(&self) -> Result<i64> {
        let n: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0))?;
        Ok(n)
    }
}

fn row_to_snapshot(row: &rusqlite::Row) -> rusqlite::Result<UsageSnapshot> {
    Ok(UsageSnapshot {
        timestamp_ms: row.get(0)?,
        session_pct: row.get::<_, f64>(1)? as f32,
        session_resets_at_ms: row.get(2)?,
        weekly_all_pct: row.get::<_, f64>(3)? as f32,
        weekly_sonnet_pct: row.get::<_, Option<f64>>(4)?.map(|v| v as f32),
        weekly_design_pct: row.get::<_, Option<f64>>(5)?.map(|v| v as f32),
        weekly_resets_at_ms: row.get(6)?,
        daily_routines_used: row.get(7)?,
        daily_routines_limit: row.get(8)?,
        raw_payload: row.get(9)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let tmp = tempfile_path();
        let db = Database::open(&tmp).unwrap();
        let snap = UsageSnapshot {
            timestamp_ms: 1_700_000_000_000,
            session_pct: 42.0,
            session_resets_at_ms: Some(1_700_018_000_000),
            weekly_all_pct: 12.3,
            weekly_sonnet_pct: Some(0.0),
            weekly_design_pct: None,
            weekly_resets_at_ms: None,
            daily_routines_used: Some(0),
            daily_routines_limit: Some(15),
            raw_payload: Some("{}".into()),
        };
        db.insert(&snap).unwrap();
        let last = db.last_snapshot().unwrap().unwrap();
        assert_eq!(last.timestamp_ms, snap.timestamp_ms);
        assert!((last.session_pct - 42.0).abs() < 0.01);
        let _ = std::fs::remove_file(&tmp);
    }

    fn tempfile_path() -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("cut-test-{}.sqlite", rand_suffix()));
        p
    }

    fn rand_suffix() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
            .to_string()
    }
}
