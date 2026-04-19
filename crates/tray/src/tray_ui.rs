//! Tray icon + menu construction and dynamic update.
//!
//! Icon is **re-rendered on every snapshot** so the bar exactly matches the
//! current percentage. The cost is negligible (32×32 RGBA = 4 KB, pure CPU).

use crate::commands::MenuItemIds;
use crate::icons::{render_bar_icon, render_unknown_icon};
use anyhow::{Context, Result};
use claude_usage_tray_core::UsageSnapshot;
use tray_icon::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

pub struct TrayUi {
    tray: TrayIcon,
    autostart_item: CheckMenuItem,
    pub ids: MenuItemIds,
}

impl TrayUi {
    pub fn build(autostart_enabled: bool) -> Result<Self> {
        let menu = Menu::new();
        let refresh = MenuItem::new("Refresh now", true, None);
        let stats = MenuItem::new("View stats…", true, None);
        let claude_settings = MenuItem::new("Open Claude settings", true, None);
        let autostart = CheckMenuItem::new("Start with Windows", true, autostart_enabled, None);
        let check_updates = MenuItem::new("Check for updates…", true, None);
        let sep = PredefinedMenuItem::separator();
        let quit = MenuItem::new("Quit", true, None);

        menu.append(&refresh)?;
        menu.append(&stats)?;
        menu.append(&claude_settings)?;
        menu.append(&sep)?;
        menu.append(&autostart)?;
        menu.append(&check_updates)?;
        menu.append(&sep)?;
        menu.append(&quit)?;

        let initial_icon = render_unknown_icon().context("render unknown icon")?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_icon(initial_icon)
            .with_tooltip("Claude Code — waiting for first sample…")
            .with_title("claude-usage-tray")
            .build()
            .context("build tray icon")?;

        let ids = MenuItemIds {
            refresh: refresh.id().0.clone(),
            stats: stats.id().0.clone(),
            claude_settings: claude_settings.id().0.clone(),
            autostart: autostart.id().0.clone(),
            check_updates: check_updates.id().0.clone(),
            quit: quit.id().0.clone(),
        };

        Ok(Self {
            tray,
            autostart_item: autostart,
            ids,
        })
    }

    pub fn apply_snapshot(&mut self, snap: &UsageSnapshot) {
        if let Ok(icon) = render_bar_icon(snap.session_pct, snap.color()) {
            let _ = self.tray.set_icon(Some(icon));
        }
        let _ = self.tray.set_tooltip(Some(snap.tooltip()));
    }

    pub fn show_rate_limited(&mut self, retry_after_secs: u64) {
        if let Ok(icon) = render_unknown_icon() {
            let _ = self.tray.set_icon(Some(icon));
        }
        let _ = self.tray.set_tooltip(Some(format!(
            "Claude Code\nRate-limited — retry in ~{}min",
            retry_after_secs / 60
        )));
    }

    pub fn show_error(&mut self, msg: &str) {
        if let Ok(icon) = render_unknown_icon() {
            let _ = self.tray.set_icon(Some(icon));
        }
        // Windows tray tooltip caps around 128 chars. Leave ~20 for the "Claude Code\nError: " prefix.
        let short: String = msg.chars().take(108).collect();
        let _ = self
            .tray
            .set_tooltip(Some(format!("Claude Code\nError: {}", short)));
    }

    pub fn set_autostart_checked(&mut self, checked: bool) {
        self.autostart_item.set_checked(checked);
    }
}
