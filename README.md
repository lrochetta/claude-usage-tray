# claude-usage-tray

> Windows system tray indicator for Claude Code session + weekly usage %. Live color-coded dot next to your clock, local SQLite history, interactive stats.

[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![npm](https://img.shields.io/npm/v/claude-usage-tray)](https://www.npmjs.com/package/claude-usage-tray)
[![Windows](https://img.shields.io/badge/platform-Windows-0078D6)](#)

---

## What it does

- 🟢 **Colored dot** in the Windows tray that reflects your current 5-hour session usage (🟢 <50% → 🟡 50–80% → 🟠 80–95% → 🔴 >95%)
- 🕑 **Tooltip** on hover: current session %, reset countdown, weekly % across all models
- 📊 **Stats report** on right-click "View stats…": line chart of session % over time, hourly heatmap, velocity (%/h), ETA-to-100%
- 💾 **Local SQLite** stores every snapshot — all your historical usage is yours
- 🚀 **Optional autostart** with Windows (HKCU registry, no admin needed)

## Quick start

```bash
npx claude-usage-tray
```

Or grab the binary directly from the [Releases page](https://github.com/lrochetta/claude-usage-tray/releases).

## How it gets the data

This tool reads your Claude Code CLI's local OAuth credentials (`~/.claude/.credentials.json`) and calls Anthropic's undocumented usage endpoint (the same one the Claude desktop app uses for the Settings → Usage view).

- **No new credentials** — it piggy-backs on your existing Claude Code auth.
- **No data leaves your machine** except the usage check itself, directly to `api.anthropic.com`.
- **No telemetry**, no analytics server.

⚠️ The endpoint is undocumented. It may be removed or changed by Anthropic without notice. Rate limits are aggressive (~5 calls, then 429 for ~30 min) — polling is intentionally throttled to **every 5 minutes**.

## Requirements

- Windows 10 / 11 (Linux / macOS builds are best-effort)
- Claude Code CLI installed and logged in (`~/.claude/.credentials.json` present)

## Configuration

The first run creates `%APPDATA%\claude-usage-tray\config.toml`. You can edit:

```toml
api_poll_secs = 300          # minimum 300 — do not lower
local_poll_secs = 60
retention_days = 90
autostart = false
alert_threshold_pct = 90.0
```

Database lives at `%APPDATA%\claude-usage-tray\usage.sqlite` (WAL mode, readable by other SQLite tools).

## Build from source

```bash
cargo build --release --bin claude-usage-tray
# Output: target\release\claude-usage-tray.exe
```

## Architecture

- `crates/core` — domain types, SQLite storage, OAuth fetch, analytics queries, HTML report generation. Pure Rust lib, UI-agnostic.
- `crates/tray` — winit event loop + `tray-icon` integration, polling thread, tooltip/icon updates.
- `npm/` — thin Node wrapper that downloads the platform binary on install and spawns it via `npx`.

```
┌─ Main thread (winit event loop) ───────────────────────┐
│   TrayIcon + Menu + UserEvent dispatch                 │
└─────────▲───────────────────────────▲──────────────────┘
          │ UserEvent::Poller(snap)   │ Menu / Tray events
          │                           │
┌─────────┴───────────┐  ┌────────────┴────────────────┐
│ Update forwarder    │  │ tray-icon global channels   │
│ (update_rx → proxy) │  └─────────────────────────────┘
└─────────▲───────────┘
          │ PollerUpdate
┌─────────┴───────────────────────────────────────────────┐
│ Poller thread                                           │
│  - Fetch usage every ≥5 min (API)                       │
│  - Insert into SQLite                                   │
│  - Backoff exponentially on 429                         │
└─────────────────────────────────────────────────────────┘
```

## License

MIT © 2026 Laurent Rochetta

## Acknowledgments

- [tray-icon](https://github.com/tauri-apps/tray-icon) by the Tauri team
- [claude-code-statusline](https://github.com/ohugonnot/claude-code-statusline) for pioneering the OAuth usage endpoint
- [ccusage](https://github.com/ryoppippi/ccusage) for the JSONL-based estimation approach
