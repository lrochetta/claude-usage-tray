# CLAUDE.md — claude-usage-tray

**Project root**: `D:\travail\DEV\claude-usage-tray\`
**Owner**: laurent (see `~/.claude/CLAUDE.md` for global context + CTO identity)
**Status**: `v0.1.0` published on GitHub + npm, 2026-04-19.

---

## 🚨 Règle #0 — Antigravity (rappel)

Ce dossier **est autorisé** en écriture durable (listé dans `D:\travail\DEV\agentic\orchestrator\CTO-CHARTER.md`). Pas besoin de redemander pour `cargo fmt`, `git commit` local, éditer des fichiers du code source, ou appliquer des fixes.

**Interdits sans GO explicite** : `git push`, `git push --tags`, `gh repo ...`, `npm publish`, actions qui modifient GitHub/npm publiquement.

## 🎯 Ce que fait ce projet

App Windows en barre système qui affiche le % de conso Claude Code (session + weekly) en live, avec une icône-jauge horizontale à côté de l'horloge. Clic-droit : refresh, modale stats (Chart.js in-app via wry), page Claude settings embarquée avec cookies persistés, autostart au démarrage de Windows.

**Raison d'être** : laurent run out de budget session Claude Code → il veut la conso toujours visible sans ouvrir l'app desktop.

**Distribution** : public npm (`npx claude-usage-tray`) + GitHub Releases (.exe Windows-x64). Source MIT. Un variant interne non-publié est à `D:\travail\DEV\agentic\tools\claude-usage-tray\`.

## 🏗 Architecture

Workspace Cargo à 2 crates :

```
claude-usage-tray/
├── crates/core/             lib — model, config, storage (SQLite), fetch, analytics, report
└── crates/tray/             bin Windows — tray-icon + winit + wry webview modals
```

Pile :
- **Rust** 1.80+, Edition 2021
- **Tray** : `tray-icon 0.19` + `winit 0.30` (ApplicationHandler, EventLoopProxy)
- **Webviews** : `wry 0.55` (WebView2 sur Windows) — stats modal + Claude settings embed avec profil persistant
- **HTTP** : `ureq 2.x` (blocking, pas de tokio)
- **DB** : `rusqlite 0.32 (bundled, WAL)` à `%APPDATA%\laurent\claude-usage-tray\data\usage.sqlite`
- **Time** : `jiff 0.1`
- **Autostart** : `auto-launch 0.5` (HKCU)
- **Icône** : procédural RGBA 32×32 (pas de crate `image`)

CI : `.github/workflows/ci.yml` (windows-latest only, wry n'a pas de deps linux dans le matrix). Release : `.github/workflows/release.yml` (tag `v*.*.*` → build Windows-x64 + GH Release + `npm publish`).

## 🗂 Fichiers clés

| Fichier | Rôle |
|---|---|
| `crates/core/src/fetch.rs` | Parser nested `{five_hour:{utilization,resets_at}, seven_day:{...}, seven_day_sonnet, seven_day_opus, seven_day_omelette,...}`. Test `parse_real_nested_response` verrouille la wire shape captée. |
| `crates/core/src/model.rs` | `UsageSnapshot`, `ThresholdColor`, tooltip rendering |
| `crates/core/src/storage.rs` | SQLite schema + insert/query |
| `crates/core/src/analytics.rs` | Time-series summary (deltas, vitesse, heures) |
| `crates/core/src/report.rs` | HTML stats (Chart.js CDN) |
| `crates/tray/src/icons.rs` | `render_bar_icon(pct, color)` — jauge horizontale |
| `crates/tray/src/tray_ui.rs` | Menu + tooltip + régén icône per snapshot |
| `crates/tray/src/poller.rs` | Thread polling 5min + backoff 429 |
| `crates/tray/src/webviews.rs` | `SubWindow` (wry) — modales stats + settings |
| `crates/tray/src/main.rs` | winit ApplicationHandler, routing events, close handling |

## 🛠 Commandes usuelles

```bash
# Build
cargo build --release --bin claude-usage-tray

# Test
cargo test --workspace

# Format + lint
cargo fmt --all && cargo clippy --workspace --all-targets

# Lancer
target/release/claude-usage-tray.exe

# Observer la DB en live
python -c "import sqlite3; c=sqlite3.connect(r'C:\\Users\\laurent\\AppData\\Roaming\\laurent\\claude-usage-tray\\data\\usage.sqlite'); print(list(c.execute('SELECT ts_ms,session_pct,weekly_all_pct FROM snapshots ORDER BY ts_ms DESC LIMIT 5')))"
```

## 🔐 Credentials

Tout dans `.credentials/` (gitignored) :
- `npm-token.json` : Granular token `claude-usage-tray CI` (Bypass 2FA ✓, expire **2026-07-18**), + ancien token révoqué + état du compte npm. Registry auth = mêmes infos dans `C:\Users\laurent\.npmrc`.

Side-car : `D:\travail\DEV\agentic\nestor\.credentials\` a les autres creds laurent (api-keys.json, ftp-nestor-sh.json).

## 📍 État actuel (2026-04-19)

- ✅ v0.1.0 publié — https://github.com/lrochetta/lrochetta/claude-usage-tray + `npm i -g claude-usage-tray`
- ✅ Binaire Windows-x64 dans la GH Release
- ✅ Internal variant (`D:\travail\DEV\agentic\tools\claude-usage-tray\`) compile (one-shot CLI, orchestrator-memory appender)
- ✅ Runtime validé live : tray tourne, DB accumule les snapshots correctement, parser nested OK

## 🚀 Pour reprendre

1. Lire d'abord `.claude-context/HANDOFF.md` → récit complet du dev + décisions + pièges
2. Puis `.claude-context/ROADMAP.md` → backlog ordonné par priorité
3. Session orchestrateur associée : `D:\travail\DEV\agentic\orchestrator\sessions\2026-04-19-claude-usage-tray-phase-ab.md`

## ⚙ Préférences de travail (rappel global)

- Français user-facing, English code/commits/docs
- Terse : action d'abord, pas de préambule
- Parallèle + ultrathink par défaut
- Ne pas parler sauf blocage dur, décision à fort impact, ou question explicite
- **Jamais** de `Co-Authored-By` dans les commits
- **Jamais** de `git commit`/`push`/`npm publish` sans GO explicite
