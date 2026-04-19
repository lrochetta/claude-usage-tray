//! Self-update: query GitHub Releases for a newer version, optionally
//! prompt the user via a wry modal, then replace the running binary.
//!
//! Uses the `self_update` crate with its `rustls` backend to avoid an
//! OpenSSL dep on Windows. The release workflow uploads a raw `.exe`
//! asset per target; `self_replace` swaps it in-place on disk.
//!
//! Threading: `check_for_update` hits the network synchronously and
//! MUST run on a worker thread — never on the winit event loop.

use anyhow::{anyhow, Context, Result};
use semver::Version;
use std::path::PathBuf;

const REPO_OWNER: &str = "lrochetta";
const REPO_NAME: &str = "claude-usage-tray";

#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub download_url: String,
    pub asset_name: String,
    pub release_notes: String,
    pub release_url: String,
    pub published_at: Option<String>,
}

/// Windows x64 asset name pattern we upload from release.yml.
fn expected_asset_name(version: &str) -> String {
    // v0.1.1 → claude-usage-tray-0.1.1-x86_64-pc-windows-msvc.exe
    let v = version.strip_prefix('v').unwrap_or(version);
    format!("claude-usage-tray-{}-x86_64-pc-windows-msvc.exe", v)
}

/// Query the GitHub Releases API for the latest release. Returns
/// `Some(UpdateInfo)` only if the latest version is strictly newer than
/// `current_version`, otherwise `None`.
pub fn check_for_update(current_version: &str) -> Result<Option<UpdateInfo>> {
    let releases = self_update::backends::github::ReleaseList::configure()
        .repo_owner(REPO_OWNER)
        .repo_name(REPO_NAME)
        .build()
        .context("configure release list")?
        .fetch()
        .context("fetch releases")?;

    let Some(latest) = releases.first() else {
        return Ok(None);
    };

    let current = Version::parse(current_version).context("parse current version")?;
    let latest_ver = Version::parse(latest.version.trim_start_matches('v'))
        .context("parse latest release version tag")?;
    if latest_ver <= current {
        return Ok(None);
    }

    let wanted = expected_asset_name(&latest.version);
    let asset = latest
        .assets
        .iter()
        .find(|a| a.name == wanted)
        .ok_or_else(|| anyhow!("no matching asset {wanted} in latest release"))?;

    Ok(Some(UpdateInfo {
        current: current.to_string(),
        latest: latest_ver.to_string(),
        download_url: asset.download_url.clone(),
        asset_name: asset.name.clone(),
        release_notes: latest.body.clone().unwrap_or_default(),
        release_url: format!(
            "https://github.com/{}/{}/releases/tag/v{}",
            REPO_OWNER, REPO_NAME, latest_ver
        ),
        published_at: Some(latest.date.clone()).filter(|s| !s.is_empty()),
    }))
}

/// Download the new binary to a temp file and swap the running binary
/// using `self_replace`. Caller should exit the process after this call
/// so the OS picks up the new binary on next launch.
pub fn install_update(info: &UpdateInfo) -> Result<()> {
    let tmp_dir = std::env::temp_dir().join("claude-usage-tray-update");
    std::fs::create_dir_all(&tmp_dir).context("create temp dir")?;
    let tmp_bin: PathBuf = tmp_dir.join(&info.asset_name);

    tracing::info!(url = %info.download_url, target = %tmp_bin.display(), "downloading update");

    {
        let file = std::fs::File::create(&tmp_bin).context("create temp binary file")?;
        self_update::Download::from_url(&info.download_url)
            .show_progress(false)
            .download_to(&file)
            .context("download release asset")?;
    }

    // Replace the running binary atomically.
    tracing::info!(src = %tmp_bin.display(), "self_replace");
    self_update::self_replace::self_replace(&tmp_bin).context("self_replace new binary")?;

    // Clean up temp file (best-effort; self_replace may have consumed it).
    let _ = std::fs::remove_file(&tmp_bin);
    let _ = std::fs::remove_dir_all(&tmp_dir);

    Ok(())
}

/// Render the update modal HTML from the update info.
pub fn render_update_html(info: &UpdateInfo) -> String {
    // Very small, self-contained HTML. No CDN calls.
    let notes_escaped = html_escape(&info.release_notes);
    let date = info.published_at.as_deref().unwrap_or("");
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>Update available</title>
<style>
  :root {{ color-scheme: dark; }}
  * {{ box-sizing: border-box; }}
  body {{
    font-family: -apple-system, "Segoe UI", system-ui, sans-serif;
    background: #0f172a;
    color: #e2e8f0;
    margin: 0;
    padding: 24px;
    line-height: 1.5;
  }}
  h1 {{ font-size: 18px; margin: 0 0 4px 0; }}
  .sub {{ color: #94a3b8; font-size: 13px; margin-bottom: 16px; }}
  .notes-title {{ font-size: 13px; text-transform: uppercase; letter-spacing: .05em; color: #94a3b8; margin: 16px 0 6px; }}
  pre {{
    background: #1e293b;
    border: 1px solid #334155;
    border-radius: 6px;
    padding: 12px;
    max-height: 340px;
    overflow-y: auto;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 12.5px;
    margin: 0;
  }}
  .buttons {{
    margin-top: 20px;
    display: flex;
    gap: 10px;
    justify-content: flex-end;
  }}
  button {{
    padding: 9px 16px;
    border: 0;
    border-radius: 6px;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    transition: background .15s;
  }}
  button.primary   {{ background: #2563eb; color: white; }}
  button.primary:hover   {{ background: #1d4ed8; }}
  button.secondary {{ background: #334155; color: #e2e8f0; }}
  button.secondary:hover {{ background: #475569; }}
  a {{ color: #60a5fa; }}
</style>
</head>
<body>
  <h1>Update available — v{latest}</h1>
  <div class="sub">You are on v{current}.{date_suffix} <a href="{url}" target="_blank">Open release page</a></div>
  <div class="notes-title">Release notes</div>
  <pre>{notes}</pre>
  <div class="buttons">
    <button class="secondary" onclick="window.ipc.postMessage('later')">Later</button>
    <button class="primary"   onclick="window.ipc.postMessage('install')">Install &amp; restart</button>
  </div>
  <script>
    // Focus the primary button for keyboard-only use.
    document.querySelector('.primary').focus();
  </script>
</body>
</html>
"##,
        latest = info.latest,
        current = info.current,
        date_suffix = if date.is_empty() {
            String::new()
        } else {
            format!(" Released {date}.")
        },
        url = info.release_url,
        notes = if notes_escaped.trim().is_empty() {
            "(no notes provided)".to_string()
        } else {
            notes_escaped
        },
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// "Up to date" toast HTML, shown when the user manually clicks "Check for updates…"
/// and we're already on the latest version.
pub fn render_uptodate_html(version: &str) -> String {
    format!(
        r##"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><title>Up to date</title>
<style>
body {{ font-family: -apple-system, "Segoe UI", system-ui, sans-serif; background: #0f172a; color: #e2e8f0; margin: 0; padding: 28px; text-align: center; }}
h1 {{ font-size: 18px; margin: 0 0 6px; }}
.sub {{ color: #94a3b8; font-size: 13px; }}
button {{ margin-top: 20px; padding: 9px 18px; border: 0; border-radius: 6px; background: #334155; color: #e2e8f0; font-size: 13px; cursor: pointer; }}
</style></head>
<body>
<h1>✓ Up to date</h1>
<div class="sub">claude-usage-tray v{ver} is the latest version.</div>
<button onclick="window.ipc.postMessage('close')">OK</button>
</body></html>"##,
        ver = version
    )
}
