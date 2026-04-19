#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! claude-usage-tray — Windows system tray indicator for Claude Code usage.

mod autostart;
mod commands;
mod icons;
mod poller;
mod tray_ui;
mod webviews;

use anyhow::{Context, Result};
use claude_usage_tray_core::{analytics, report, Config, Database};
use commands::Command;
use crossbeam_channel::{unbounded, Sender};
use poller::{PollerCommand, PollerUpdate};
use std::path::PathBuf;
use tracing::{error, info, warn};
use tray_icon::menu::MenuEvent;
use tray_icon::TrayIconEvent;
use tray_ui::TrayUi;
use webviews::SubWindow;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

#[derive(Debug)]
enum UserEvent {
    Menu(MenuEvent),
    Tray(TrayIconEvent),
    Poller(PollerUpdate),
}

struct App {
    db_path: PathBuf,
    webview_data_dir: PathBuf,
    ui: Option<TrayUi>,
    poller_cmd_tx: Sender<PollerCommand>,
    stats_window: Option<SubWindow>,
    settings_window: Option<SubWindow>,
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {
        if self.ui.is_none() {
            match TrayUi::build(autostart::is_enabled()) {
                Ok(ui) => {
                    self.ui = Some(ui);
                    info!("tray icon initialized");
                }
                Err(e) => {
                    error!("failed to build tray UI: {:#}", e);
                }
            }
        }
    }

    fn window_event(&mut self, _el: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if matches!(event, WindowEvent::CloseRequested) {
            if self.stats_window.as_ref().is_some_and(|w| w.id() == id) {
                self.stats_window = None;
                return;
            }
            if self.settings_window.as_ref().is_some_and(|w| w.id() == id) {
                self.settings_window = None;
            }
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Menu(me) => self.handle_menu(event_loop, me),
            UserEvent::Tray(_te) => {
                // Reserved for future: left-click to open the stats modal.
            }
            UserEvent::Poller(u) => self.handle_poller_update(u),
        }
    }
}

impl App {
    fn handle_menu(&mut self, event_loop: &ActiveEventLoop, evt: MenuEvent) {
        let Some(ui) = self.ui.as_mut() else { return };
        let Some(cmd) = ui.ids.resolve(&evt.id.0) else {
            return;
        };
        match cmd {
            Command::RefreshNow => {
                let _ = self.poller_cmd_tx.send(PollerCommand::RefreshNow);
            }
            Command::OpenStats => {
                if let Some(win) = &self.stats_window {
                    win.focus();
                    return;
                }
                match build_stats_html(&self.db_path) {
                    Ok(html) => match webviews::build_stats_window(event_loop, &html) {
                        Ok(sw) => self.stats_window = Some(sw),
                        Err(e) => {
                            error!("stats webview failed: {:#}", e);
                            warn!("falling back to browser");
                            let _ = open_stats_in_browser(&html);
                            ui.show_error(&format!("stats modal: {}", e));
                        }
                    },
                    Err(e) => {
                        error!("build stats html failed: {:#}", e);
                        ui.show_error(&format!("stats: {}", e));
                    }
                }
            }
            Command::OpenClaudeSettings => {
                if let Some(win) = &self.settings_window {
                    win.focus();
                    return;
                }
                match webviews::build_settings_window(event_loop, self.webview_data_dir.clone()) {
                    Ok(sw) => self.settings_window = Some(sw),
                    Err(e) => {
                        error!("settings webview failed: {:#}", e);
                        warn!("falling back to browser");
                        let _ = open::that("https://claude.ai/settings/usage")
                            .or_else(|_| open::that("https://claude.ai/settings"));
                    }
                }
            }
            Command::ToggleAutostart => match autostart::toggle() {
                Ok(new_state) => ui.set_autostart_checked(new_state),
                Err(e) => {
                    error!("autostart toggle failed: {:#}", e);
                    ui.show_error(&format!("autostart: {}", e));
                }
            },
            Command::Quit => {
                let _ = self.poller_cmd_tx.send(PollerCommand::Quit);
                event_loop.exit();
            }
        }
    }

    fn handle_poller_update(&mut self, u: PollerUpdate) {
        let Some(ui) = self.ui.as_mut() else { return };
        match u {
            PollerUpdate::Snapshot(s) => ui.apply_snapshot(&s),
            PollerUpdate::RateLimited { retry_after_secs } => {
                ui.show_rate_limited(retry_after_secs)
            }
            PollerUpdate::Error(e) => ui.show_error(&e),
        }
    }
}

fn build_stats_html(db_path: &PathBuf) -> Result<String> {
    let db = Database::open_readonly(db_path).context("open db readonly")?;
    let summary = analytics::summary(&db, 7 * 24)?;
    Ok(report::render_html(&summary))
}

fn open_stats_in_browser(html: &str) -> Result<()> {
    let mut out = std::env::temp_dir();
    out.push("claude-usage-tray-stats.html");
    std::fs::write(&out, html)?;
    open::that(&out)?;
    Ok(())
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "claude-usage-tray starting"
    );

    let cfg = Config::load_or_default().context("load config")?;
    let db_path = Config::database_path().context("resolve db path")?;
    let webview_data_dir = db_path
        .parent()
        .map(|p| p.join("webview-data"))
        .unwrap_or_else(|| std::env::temp_dir().join("claude-usage-tray-webview"));

    // Ensure DB exists up-front so readonly opens later don't race.
    drop(Database::open(&db_path).context("init db")?);

    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .context("build event loop")?;
    let proxy = event_loop.create_proxy();

    // Wire tray crate events → our UserEvent channel.
    {
        let proxy = proxy.clone();
        MenuEvent::set_event_handler(Some(move |e| {
            let _ = proxy.send_event(UserEvent::Menu(e));
        }));
    }
    {
        let proxy = proxy.clone();
        TrayIconEvent::set_event_handler(Some(move |e| {
            let _ = proxy.send_event(UserEvent::Tray(e));
        }));
    }

    // Spawn poller.
    let (cmd_tx, cmd_rx) = unbounded::<PollerCommand>();
    let (update_tx, update_rx) = unbounded::<PollerUpdate>();
    let _poller_handle = poller::spawn(cfg.clone(), db_path.clone(), cmd_rx, update_tx);

    // Forward poller updates → UserEvent via proxy.
    {
        let proxy = proxy.clone();
        std::thread::Builder::new()
            .name("update-forwarder".into())
            .spawn(move || {
                while let Ok(update) = update_rx.recv() {
                    if proxy.send_event(UserEvent::Poller(update)).is_err() {
                        break;
                    }
                }
            })
            .context("spawn update forwarder")?;
    }

    // Request first refresh immediately.
    let _ = cmd_tx.send(PollerCommand::RefreshNow);

    let mut app = App {
        db_path,
        webview_data_dir,
        ui: None,
        poller_cmd_tx: cmd_tx,
        stats_window: None,
        settings_window: None,
    };

    event_loop.run_app(&mut app).context("run event loop")?;
    Ok(())
}
