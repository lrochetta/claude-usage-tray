//! Render the tray icon as a horizontal progress bar proportional to
//! session % usage. Colored by threshold.
//!
//! Windows scales this down to ~16×16 at 100% DPI, ~24×24 at 150%.
//! We render a 32×32 canvas — big enough to look crisp on high DPI,
//! and a solid bar reads cleanly even when downscaled.

use claude_usage_tray_core::model::ThresholdColor;

const ICON_SIZE: u32 = 32;

/// Build a `tray_icon::Icon` depicting a horizontal progress bar filled
/// to `pct` percent in `color`.
pub fn render_bar_icon(pct: f32, color: ThresholdColor) -> anyhow::Result<tray_icon::Icon> {
    let rgba = render_bar(pct, color);
    let icon = tray_icon::Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE)?;
    Ok(icon)
}

/// Neutral "no data yet" icon — empty outlined bar.
pub fn render_unknown_icon() -> anyhow::Result<tray_icon::Icon> {
    let rgba = render_bar_empty();
    let icon = tray_icon::Icon::from_rgba(rgba, ICON_SIZE, ICON_SIZE)?;
    Ok(icon)
}

fn render_bar(pct: f32, color: ThresholdColor) -> Vec<u8> {
    let [r, g, b, _] = color.rgba();
    draw(pct.clamp(0.0, 100.0) / 100.0, [r, g, b, 0xFF], false)
}

fn render_bar_empty() -> Vec<u8> {
    draw(0.0, [0x64, 0x74, 0x8B, 0xFF], true)
}

/// Draw a rounded-corner outlined rectangle with a horizontal fill.
/// `fill_ratio` in 0..=1 controls how far from the left the filled color extends.
fn draw(fill_ratio: f32, fill_color: [u8; 4], is_empty_state: bool) -> Vec<u8> {
    let size = ICON_SIZE as i32;
    let mut buf = vec![0u8; (size * size * 4) as usize];

    // Layout: outer border, 2-px padding on top and bottom so the bar is tall
    // but not full-height (easier to recognize as a "gauge" at small sizes).
    let outer_pad_y: i32 = 6; // 32 - 6*2 = 20-row bar
    let outer_pad_x: i32 = 2;
    let bar_left = outer_pad_x;
    let bar_right = size - outer_pad_x - 1;
    let bar_top = outer_pad_y;
    let bar_bottom = size - outer_pad_y - 1;
    let bar_width = (bar_right - bar_left) as f32;

    // Filled pixel column index (0-based, inclusive).
    let filled_end = bar_left + (fill_ratio * bar_width).round() as i32;

    let bg_empty = [0x20, 0x27, 0x39, 0xFF];
    let border = [0xE2, 0xE8, 0xF0, 0xFF];

    for y in 0..size {
        for x in 0..size {
            let idx = ((y * size + x) * 4) as usize;
            let in_bar = x >= bar_left && x <= bar_right && y >= bar_top && y <= bar_bottom;
            if !in_bar {
                continue; // transparent
            }
            let is_border = x == bar_left || x == bar_right || y == bar_top || y == bar_bottom;
            let inner_x = x;
            let inner_y = y;
            let is_corner = (inner_x == bar_left || inner_x == bar_right)
                && (inner_y == bar_top || inner_y == bar_bottom);
            if is_corner {
                // Rounded: drop the corner pixels.
                continue;
            }
            let p = if is_border {
                border
            } else if !is_empty_state && x <= filled_end {
                fill_color
            } else {
                bg_empty
            };
            buf[idx] = p[0];
            buf[idx + 1] = p[1];
            buf[idx + 2] = p[2];
            buf[idx + 3] = p[3];
        }
    }
    buf
}
