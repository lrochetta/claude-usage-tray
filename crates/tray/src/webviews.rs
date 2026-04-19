//! In-app webview windows backed by WebView2 (via `wry`).
//!
//! We expose two windows:
//!
//! * **Stats** — hosts the HTML report produced by `report::render_html`.
//! * **Claude settings** — embeds `https://claude.ai/settings/usage` with a
//!   persistent WebView2 user-data folder, so once the user logs in once,
//!   cookies survive app restarts.
//!
//! Field drop order matters: `webview` must drop **before** `window` so the
//! underlying HWND is still alive when the WebView releases its controller.

use anyhow::{Context, Result};
use std::path::PathBuf;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes, WindowId};
use wry::{WebContext, WebView, WebViewBuilder};

pub struct SubWindow {
    // Drop order: webview first, then window. Do not reorder.
    // `webview` is held only for its Drop side-effect (releasing the WebView2
    // controller before the HWND dies); we never call methods on it after build.
    #[allow(dead_code)]
    pub webview: WebView,
    pub window: Window,
}

impl SubWindow {
    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    pub fn focus(&self) {
        self.window.focus_window();
    }
}

/// Anchor the modal at the bottom-right of the primary monitor, just above
/// the Windows taskbar. Uses a conservative 48 logical-px taskbar estimate —
/// accurate enough in practice.
fn bottom_right(
    event_loop: &ActiveEventLoop,
    size: LogicalSize<f64>,
) -> Option<PhysicalPosition<i32>> {
    let monitor = event_loop
        .primary_monitor()
        .or_else(|| event_loop.available_monitors().next())?;
    let scale = monitor.scale_factor();
    let pos = monitor.position();
    let mon = monitor.size();
    let phys_w = (size.width * scale) as i32;
    let phys_h = (size.height * scale) as i32;
    let margin = (12.0 * scale) as i32;
    let taskbar = (48.0 * scale) as i32;
    let x = pos.x + mon.width as i32 - phys_w - margin;
    let y = pos.y + mon.height as i32 - phys_h - taskbar - margin;
    Some(PhysicalPosition::new(x, y))
}

pub fn build_stats_window(event_loop: &ActiveEventLoop, html: &str) -> Result<SubWindow> {
    let size = LogicalSize::new(560.0, 680.0);
    let mut attrs = WindowAttributes::default()
        .with_title("Claude Code — stats")
        .with_inner_size(size)
        .with_resizable(true);
    if let Some(p) = bottom_right(event_loop, size) {
        attrs = attrs.with_position(p);
    }
    let window = event_loop
        .create_window(attrs)
        .context("create stats window")?;
    let webview = WebViewBuilder::new()
        .with_html(html)
        .build(&window)
        .context("build stats webview")?;
    Ok(SubWindow { webview, window })
}

pub fn build_settings_window(event_loop: &ActiveEventLoop, data_dir: PathBuf) -> Result<SubWindow> {
    let size = LogicalSize::new(980.0, 740.0);
    let mut attrs = WindowAttributes::default()
        .with_title("Claude — settings")
        .with_inner_size(size)
        .with_resizable(true);
    if let Some(p) = bottom_right(event_loop, size) {
        attrs = attrs.with_position(p);
    }
    let window = event_loop
        .create_window(attrs)
        .context("create settings window")?;
    let _ = std::fs::create_dir_all(&data_dir);
    // WebContext configures the WebView2 user-data folder. Once the WebView
    // is built, the folder path is copied into the WebView2 environment;
    // dropping the context afterwards is safe.
    let mut ctx = WebContext::new(Some(data_dir));
    let webview = WebViewBuilder::new_with_web_context(&mut ctx)
        .with_url("https://claude.ai/settings/usage")
        .build(&window)
        .context("build settings webview")?;
    Ok(SubWindow { webview, window })
}
