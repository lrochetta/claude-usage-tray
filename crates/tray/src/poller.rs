//! Background poller — fetches usage on a cadence, inserts into DB, notifies UI.
//!
//! Two cadences:
//!   - API poll (expensive, rate-limited)   — `config.effective_api_poll_secs()`
//!   - Local estimation (cheap, JSONL scan) — `config.effective_local_poll_secs()`
//!
//! v0.1: local estimation is a stub that re-uses the last API snapshot's timestamp.
//! Future work: tail `~/.claude/projects/**/*.jsonl` to estimate fine-grained tokens.

use claude_usage_tray_core::{Config, CoreError, Database, UsageSnapshot};
use crossbeam_channel::{select, Receiver, Sender};
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub enum PollerCommand {
    RefreshNow,
    Quit,
}

#[derive(Debug, Clone)]
pub enum PollerUpdate {
    Snapshot(UsageSnapshot),
    Error(String),
    RateLimited { retry_after_secs: u64 },
}

pub fn spawn(
    cfg: Config,
    db_path: PathBuf,
    cmd_rx: Receiver<PollerCommand>,
    update_tx: Sender<PollerUpdate>,
) -> std::thread::JoinHandle<()> {
    std::thread::Builder::new()
        .name("usage-poller".into())
        .spawn(move || run(cfg, db_path, cmd_rx, update_tx))
        .expect("failed to spawn poller thread")
}

fn run(
    cfg: Config,
    db_path: PathBuf,
    cmd_rx: Receiver<PollerCommand>,
    update_tx: Sender<PollerUpdate>,
) {
    let db = match Database::open(&db_path) {
        Ok(d) => d,
        Err(e) => {
            let _ = update_tx.send(PollerUpdate::Error(format!("db open: {}", e)));
            return;
        }
    };

    // Initial load — surface last snapshot to UI immediately.
    if let Ok(Some(last)) = db.last_snapshot() {
        let _ = update_tx.send(PollerUpdate::Snapshot(last));
    }

    let mut next_api_poll = Instant::now();
    let mut backoff_secs: u64 = 0;

    loop {
        let now = Instant::now();
        let wait = if next_api_poll > now {
            next_api_poll - now
        } else {
            Duration::from_millis(0)
        };

        select! {
            recv(cmd_rx) -> msg => {
                match msg {
                    Ok(PollerCommand::RefreshNow) => {
                        // Fall through to immediate poll below.
                    }
                    Ok(PollerCommand::Quit) => {
                        tracing::info!("poller: quit received");
                        return;
                    }
                    Err(_) => return,
                }
            }
            default(wait) => {
                // Time to poll API
            }
        }

        // --- Perform API poll ---
        match poll_once(&cfg, &db) {
            Ok(snap) => {
                backoff_secs = 0;
                let _ = update_tx.send(PollerUpdate::Snapshot(snap));
                next_api_poll = Instant::now() + Duration::from_secs(cfg.effective_api_poll_secs());
            }
            Err(CoreError::RateLimited { retry_after_secs }) => {
                tracing::warn!(retry_after_secs, "poller: rate limited");
                let _ = update_tx.send(PollerUpdate::RateLimited { retry_after_secs });
                // Exponential backoff capped at 1h
                backoff_secs = if backoff_secs == 0 {
                    retry_after_secs.max(300)
                } else {
                    (backoff_secs * 2).min(3600)
                };
                next_api_poll = Instant::now() + Duration::from_secs(backoff_secs);
            }
            Err(e) => {
                tracing::warn!(error = %e, "poller: fetch failed");
                let _ = update_tx.send(PollerUpdate::Error(e.to_string()));
                // Retry after the normal interval
                next_api_poll = Instant::now() + Duration::from_secs(cfg.effective_api_poll_secs());
            }
        }
    }
}

fn poll_once(cfg: &Config, db: &Database) -> Result<UsageSnapshot, CoreError> {
    // Config-aware fetch: CLAUDE_OAUTH_TOKEN env > config.oauth_token_override
    // > ~/.claude/.credentials.json (with auto-refresh).
    let snap = claude_usage_tray_core::fetch_usage_with_config(cfg)?;
    db.insert(&snap)?;
    // Opportunistic cleanup: ~1% chance per poll
    if fastrand_1_in(100) {
        let cutoff =
            jiff::Timestamp::now().as_millisecond() - (cfg.retention_days as i64) * 86_400 * 1000;
        let _ = db.purge_older_than(cutoff);
    }
    Ok(snap)
}

fn fastrand_1_in(n: u64) -> bool {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    nanos % n == 0
}
