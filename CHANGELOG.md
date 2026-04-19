# Changelog

All notable changes to `claude-usage-tray` are documented in this file.

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), semver.

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
