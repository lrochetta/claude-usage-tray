//! Thin wrapper around `auto-launch` for Windows autostart.

use anyhow::{Context, Result};
use auto_launch::AutoLaunch;

const APP_NAME: &str = "claude-usage-tray";

fn launcher() -> Result<AutoLaunch> {
    let exe = std::env::current_exe().context("resolve current exe")?;
    let path_str = exe
        .to_str()
        .context("current exe path is not valid UTF-8")?
        .to_string();
    Ok(AutoLaunch::new(APP_NAME, &path_str, &[] as &[&str]))
}

pub fn is_enabled() -> bool {
    launcher()
        .and_then(|l| Ok(l.is_enabled()?))
        .unwrap_or(false)
}

pub fn enable() -> Result<()> {
    let l = launcher()?;
    l.enable()?;
    Ok(())
}

pub fn disable() -> Result<()> {
    let l = launcher()?;
    l.disable()?;
    Ok(())
}

pub fn toggle() -> Result<bool> {
    if is_enabled() {
        disable()?;
        Ok(false)
    } else {
        enable()?;
        Ok(true)
    }
}
