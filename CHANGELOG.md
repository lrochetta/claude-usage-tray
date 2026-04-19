# Changelog

All notable changes to `claude-usage-tray` are documented in this file.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), semver.

---

## [0.2.0] - 2026-04-19

Minor — in-app auto-update from GitHub Releases.

### Added
- **Auto-update check at startup**: queries the GitHub Releases API for the latest `lrochetta/claude-usage-tray` tag and, if strictly newer than the running version, pops an in-app modal with the release notes and Install / Later buttons.
- **Throttled startup check**: configurable interval (default 6h) via `auto_update.check_interval_hours`; timestamp stored in `auto_update.last_check_ts_ms` to survive restarts.
- **Config toggle**: `auto_update.check_enabled = false` disables all startup checks for privacy / air-gapped setups. Manual "Check for updates…" still works.
- **Manual menu item "Check for updates…"**: forces a fresh check on demand, shows either the update modal or an "Up to date" confirmation.
- **One-click install**: "Install & restart" downloads the matching Windows-x64 binary, atomically replaces the running exe via `self_replace`, then exits cleanly so Windows picks up the new binary on next launch.
- **IPC-driven modal**: update dialog uses `wry`'s `with_ipc_handler` to route button clicks back to Rust (`window.ipc.postMessage("install" | "later" | "close")`).

### Changed
- `Cargo.toml` workspace tray crate now pulls `self_update 0.42` (rustls, compression-flate2, archive-zip) and `semver 1`.
- `Config` schema: new `AutoUpdateConfig` struct with `check_enabled` (default `true`), `auto_install` (default `false`, always prompt), `last_check_ts_ms`, `check_interval_hours` (default 6). Backward-compatible via `#[serde(default)]`.

### Notes
- Current version compared against the latest tag using `semver::Version`; downgrade is never automatic.
- Update checks are silent on network failure — no tooltip noise; logged at `warn` only.
- Install fails gracefully: any error surfaces through the tray tooltip; the user can retry from the menu.

---

## [0.1.1] - 2026-04-19

Patch — better UX on PCs without Claude Code logged in.

### Added
- **`CLAUDE_OAUTH_TOKEN` env var** override: set it before launching the tray and the app skips the `.credentials.json` lookup. Useful for machines without Claude Code installed.
- **`oauth_token_override` config field** (`config.toml`): same purpose as the env var but persistent.
- New public API `claude_usage_tray_core::fetch_usage_with_config(cfg)`.

### Changed
- Tooltip tray error for missing credentials now reads **"Claude Code not logged in on this PC. Run `claude login` or set CLAUDE_OAUTH_TOKEN"** (actionable) instead of the raw path.
- Error-text budget in tooltips bumped from 80 → 108 chars so actionable hints aren't truncated.

### Known limitation
- Env / config override tokens are raw access tokens with no refresh; when they expire, the app shows the auth error until the user updates the value. Use the credentials-file path if you want transparent refresh.

---

## [0.1.0] - 2026-04-19

Initial release. Windows system tray app showing Claude Code session & weekly usage percentages. Local SQLite time-series for usage analytics.

### Added
- Color-coded tray icon (green <50% / yellow 50-80% / orange 80-95% / red >95%)
- Tooltip with session %, reset countdown, weekly all-models %
- Right-click menu: Refresh now, View stats, Open Claude settings, Start with Windows, Quit
- Polling every 5 minutes from Anthropic OAuth usage endpoint (rate-limit aware)
- Local JSONL tail polling every 1 minute for fine-grained token counts (fallback / estimation)
- SQLite storage of all snapshots at `%APPDATA%/claude-usage-tray/usage.sqlite`
- Stats report (HTML) with consumption over time, hourly heatmap, velocity %/h, ETA to 100%
- Config file at `%APPDATA%/claude-usage-tray/config.toml`
- Auto-start with Windows (HKCU registry via `auto-launch`)
- npm wrapper for `npx claude-usage-tray` installation

### Notes
- The Anthropic OAuth `/api/oauth/usage` endpoint is undocumented. May break without notice.
- Rate limit observed: ~5 requests then 429 for 30+ min — poll interval enforced at 5 min minimum.
- OAuth token piggy-backs on Claude Code CLI refresh cycle (`~/.claude/.credentials.json`).
