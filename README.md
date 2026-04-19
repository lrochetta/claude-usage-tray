# claude-usage-tray
<img width="359" height="141" alt="image" src="https://github.com/user-attachments/assets/a4e6f75f-1d21-443e-ad80-135887a62546" />


> Windows system tray indicator for Claude Code session + weekly usage %. Live color-coded gauge **with the percentage drawn on the icon**, in-app stats modal, embedded Claude settings with persistent login, and one-click auto-updates from GitHub Releases.

[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![npm](https://img.shields.io/npm/v/claude-usage-tray)](https://www.npmjs.com/package/claude-usage-tray)
[![Windows](https://img.shields.io/badge/platform-Windows-0078D6)](#)

---

## What it does

- 🎯 **Gauge icon with live percentage** — a horizontal bar that fills left-to-right as you burn through your 5-hour session budget, with the current % drawn **directly on the icon** (e.g. `38`, `72`, or `!!` at 100%). Color-coded: 🟢 <50 → 🟡 50–80 → 🟠 80–95 → 🔴 ≥95.
- 🕑 **Tooltip** on hover: session %, reset countdown, weekly % across all models, plus Sonnet / Design breakdown when non-zero.
- 📊 **In-app stats modal** (right-click → *View stats…*): line chart of session % over time, hourly heatmap, consumption velocity (%/h), ETA-to-100%. No browser needed — renders inside the app via an embedded WebView2 window.
- 🔐 **Embedded Claude settings** (right-click → *Open Claude settings*): opens `claude.ai/settings/usage` inside the app with a persistent WebView2 profile so your login survives restarts.
- 💾 **Local SQLite** at `%APPDATA%\laurent\claude-usage-tray\data\usage.sqlite` — every snapshot is yours, readable by any SQLite tool.
- 🚀 **Optional Windows autostart** (HKCU, no admin needed).
- ⬆️ **Self-update** — queries GitHub Releases on launch, pops a modal with the changelog and one-click "Install & restart".

## Quick start

```bash
npx claude-usage-tray
```

Or grab the binary directly from the [Releases page](https://github.com/lrochetta/claude-usage-tray/releases) and run it.

> **On first launch from a downloaded `.exe`** you may see a Windows SmartScreen warning (*"Windows protected your PC"*) because the binary is **not yet code-signed**. See the [Windows SmartScreen warning](#️-windows-smartscreen-warning-on-first-run) section below for how to bypass it safely and the signing roadmap.

## Requirements

- **Windows 10 / 11** (Linux / macOS builds are deferred — see the [Roadmap](#roadmap))
- **Claude Code CLI** installed and logged in on this PC (`~/.claude/.credentials.json` present). Alternative: supply a token manually, see *Credentials fallback* below.
- **WebView2 Runtime** — bundled with Windows 11 and auto-installed on Win10 since 2021, so effectively always present.

## Latest updates

### v0.3.0 — numeric percentage on the icon
The session % is now drawn as digits on top of the colored gauge (3×5 bitmap glyph atlas, ×3 scale, white with black halo). You see the number at a glance, no hover needed. At 100%+ the icon displays `!!` as an unmistakable alert.

### v0.2.0 — in-app auto-update
- Startup check against `lrochetta/claude-usage-tray` GitHub Releases (throttled 6 h).
- Modal (wry webview) with the release notes and **Install & restart** / **Later** buttons.
- Atomic in-place binary swap via `self_replace`, then clean exit → Windows picks up the new `.exe` on next launch.
- Menu item **Check for updates…** for on-demand checks.
- Opt-out via `config.toml` → `auto_update.check_enabled = false`.

### v0.1.1 — credentials fallback
Deployable on PCs without Claude Code logged in. Two escape hatches, tried before the `.credentials.json` lookup:
- `CLAUDE_OAUTH_TOKEN` env var (no refresh; update when it expires).
- `oauth_token_override = "..."` in `config.toml` (same deal, persistent).

The credentials-file path remains the recommended default because it enables transparent token refresh.

### v0.1.0 — initial release
Color gauge + tooltip, SQLite storage, Chart.js stats report, HKCU autostart, npm wrapper.

Full history: [`CHANGELOG.md`](CHANGELOG.md).

## How it gets the data

This tool reads your Claude Code CLI's local OAuth credentials (`~/.claude/.credentials.json`) and calls Anthropic's undocumented usage endpoint (the same one the Claude desktop app uses for the Settings → Usage view).

- **No new credentials** — piggy-backs on your existing Claude Code auth. The token is auto-refreshed when expired and written back to the same file so the CLI keeps working.
- **No data leaves your machine** except the usage check itself, directly to `api.anthropic.com`.
- **No telemetry**, no analytics server.

⚠️ The endpoint is undocumented. It may be removed or changed by Anthropic without notice. Rate limits are aggressive (~5 calls, then 429 for ~30 min) — polling is intentionally throttled to **every 5 minutes** with exponential backoff on 429.

### Credentials fallback (v0.1.1+)

If `~/.claude/.credentials.json` is missing (e.g. this PC doesn't have Claude Code installed), the tooltip says *"Claude Code not logged in on this PC. Run `claude login` or set CLAUDE_OAUTH_TOKEN"*. Three options, in priority order:

1. **Install + login Claude Code** (recommended — enables transparent refresh):
   ```bash
   npm i -g @anthropic-ai/claude-code
   claude login
   ```
2. **Environment variable** (raw access token, no refresh — when it expires, update it):
   ```powershell
   setx CLAUDE_OAUTH_TOKEN "sk-ant-oat01-..."
   ```
3. **Config file** (persistent, same limitation as env var):
   ```toml
   # %APPDATA%\laurent\claude-usage-tray\config\config.toml
   oauth_token_override = "sk-ant-oat01-..."
   ```

## Keeping the icon visible next to the clock

By default, Windows hides new tray icons behind the `^` overflow arrow. To pin `claude-usage-tray` permanently in the always-visible zone:

1. **Right-click** the taskbar → **Taskbar settings**
2. Scroll to **Other system tray icons**
3. Toggle **`claude-usage-tray`** to **On**

Persistent across reboots. Windows does not let applications pin themselves — this is a user-controlled setting by design.

## Menu reference

Right-click the tray icon:

| Item | What it does |
|---|---|
| **Refresh now** | Force an immediate usage fetch (respects rate limit) |
| **View stats…** | In-app modal with Chart.js graphs over the last 7 days |
| **Open Claude settings** | Embedded WebView2 to `claude.ai/settings/usage`, cookies persisted |
| **Start with Windows** (toggle) | HKCU autostart registry entry |
| **Check for updates…** | Fresh GitHub Releases query — modal if new, "up to date" toast if current |
| **Quit** | Clean shutdown (poller, windows, DB flush) |

## Configuration

The first run creates `%APPDATA%\laurent\claude-usage-tray\config\config.toml`. Defaults work for most users; these are the knobs:

```toml
api_poll_secs = 300          # API poll interval in seconds. Minimum 300 — do not lower.
local_poll_secs = 60         # Local JSONL scan cadence (cheap).
retention_days = 90          # Keep this many days of snapshots in SQLite.
autostart = false            # Redundant — toggle via the menu instead.
alert_threshold_pct = 90.0   # Reserved for future notification support.

# Point to a non-standard Claude credentials file.
# credentials_path_override = "D:/alt/.claude/.credentials.json"

# Raw OAuth token override (v0.1.1+). If non-empty, skips the credentials-file lookup.
# No refresh — update when it expires.
# oauth_token_override = "sk-ant-oat01-..."

[auto_update]
check_enabled = true         # Set to false to disable all startup update checks.
auto_install = false         # Prompt on updates (true = silent install, advanced).
last_check_ts_ms = 0         # Auto-managed timestamp of the last successful check.
check_interval_hours = 6     # Minimum hours between startup checks.
```

Database: `%APPDATA%\laurent\claude-usage-tray\data\usage.sqlite` (WAL mode). WebView2 user-data folder (for persistent Claude login cookies): `%APPDATA%\laurent\claude-usage-tray\data\webview-data\`.

## ⚠️ Windows SmartScreen warning on first run

If you download the `.exe` from [GitHub Releases](https://github.com/lrochetta/claude-usage-tray/releases) (not via `npx`), Windows may display a blue dialog:

> **Windows protected your PC**
> Microsoft Defender SmartScreen prevented an unrecognized app from starting.
> App: `claude-usage-tray-X.Y.Z-x86_64-pc-windows-msvc.exe`

This is expected. The binary is **not currently code-signed** (EV certificates cost 300–600 €/year, or require enrollment with a free OSS signing program — see roadmap below). Without a cert, SmartScreen doesn't know the publisher yet and shows the warning until the binary builds enough download reputation over time.

### How to bypass it safely

1. In the SmartScreen dialog, click **More info**.
2. Click **Run anyway**.

The warning doesn't reappear for that specific binary.

### Verifying the binary is legit

You are trusting me (`@lrochetta`), not a compiled certificate authority. Quick checks before running:

- **Download only from** https://github.com/lrochetta/claude-usage-tray/releases (the official source). Never from mirrors or random websites.
- **Source** is 100% open at this repo. Review `crates/tray/src/` if you want to audit what the tray does before launching.
- **Checksum** each release publishes a SHA-256 in the GitHub Release notes — compare with `Get-FileHash .\claude-usage-tray-X.Y.Z-x86_64-pc-windows-msvc.exe`.
- **Build from source** if you want full certainty:
  ```bash
  git clone https://github.com/lrochetta/claude-usage-tray.git
  cd claude-usage-tray
  cargo build --release --bin claude-usage-tray
  ```

### Signing roadmap

The project will apply to the [SignPath.io free OSS signing program](https://signpath.org/about) in a future release. Once accepted:

- Every binary produced by the GitHub Actions release workflow will be signed with a SignPath-issued certificate.
- First-download SmartScreen warnings should vanish within a few weeks as reputation builds.
- No warnings on subsequent releases.

For paid, zero-delay trust (commercial deployments), an EV (Extended Validation) certificate from SSL.com or similar is the alternative (~350 €/year + hardware token).

## Build from source

```bash
cargo build --release --bin claude-usage-tray
# Output: target\release\claude-usage-tray.exe
```

Run directly: `target\release\claude-usage-tray.exe`. No install, no admin, no side effects outside `%APPDATA%\laurent\claude-usage-tray\`.

## Architecture

- `crates/core` — domain types, SQLite storage, OAuth fetch with nested-payload parser, analytics queries, HTML report generator. Pure Rust lib, UI-agnostic.
- `crates/tray` — winit event loop + `tray-icon` + `wry` (WebView2) integration, polling thread, self-updater module.
- `npm/` — thin Node wrapper (`claude-usage-tray`) whose postinstall downloads the platform binary from GitHub Releases and exposes it via `npx`.

```
┌─ Main thread (winit event loop) ───────────────────────┐
│   TrayIcon + Menu + wry sub-windows                    │
│   UserEvent dispatch: Menu, Tray, Poller, Update*      │
└─────────▲───────────────────────────▲──────────────────┘
          │ UserEvent                 │ tray-icon / menu events
          │                           │ + wry IPC (update modal)
┌─────────┴──────┐  ┌────────┴─────┐  ┌──────────────────┐
│ Poller thread  │  │ Startup      │  │ Sub-windows      │
│  - fetch 5min  │  │ update check │  │  - stats modal   │
│  - SQLite      │  │ (throttled)  │  │  - claude.ai     │
│  - backoff 429 │  │              │  │  - update modal  │
└────────────────┘  └──────────────┘  └──────────────────┘
```

## Roadmap

| Milestone | Status |
|---|---|
| Cross-platform (Linux WebKitGTK, macOS WKWebView) | deferred — v0.1 is Windows-only |
| Code signing (SignPath OSS → EV cert) | planned — see *Signing roadmap* |
| Per-model analytics dashboard (Sonnet / Opus / Design) | on the wishlist |
| Desktop notifications at 80 / 90 / 95 % | planned |
| `.msi` installer for enterprise deployment | maybe |

## License

MIT © 2026 Laurent Rochetta

## Acknowledgments

- [tray-icon](https://github.com/tauri-apps/tray-icon) by the Tauri team
- [wry](https://github.com/tauri-apps/wry) for the cross-platform WebView embedding
- [self_update](https://github.com/jaemk/self_update) for the GitHub Releases auto-update plumbing
- [claude-code-statusline](https://github.com/ohugonnot/claude-code-statusline) for pioneering the OAuth usage endpoint
- [ccusage](https://github.com/ryoppippi/ccusage) for the JSONL-based estimation approach
