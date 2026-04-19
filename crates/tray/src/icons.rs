//! Tray icon renderer.
//!
//! Layout of the 32×32 RGBA buffer:
//!
//! * Full-bleed horizontal gauge bar: left `pct`% filled in `ThresholdColor`,
//!   right remainder in a neutral dark blue, 1-px border all around.
//! * Overlay: the integer percentage centered in white with a 1-px black halo
//!   for legibility against either fill or empty zones.
//!
//! At 100% DPI the Windows tray scales this down to ~16×16 — still readable
//! because the digits are drawn from a purpose-built 3×5 bitmap font at ×3
//! scale (9×15 px per glyph), which stays crisp after downscaling.

use claude_usage_tray_core::model::ThresholdColor;

const SIZE: i32 = 32;
const GLYPH_W: usize = 3;
const GLYPH_H: usize = 5;
const SCALE: i32 = 3;

/// 3×5 bitmap font, 1 byte per row, MSB-left. Only digits — nothing else
/// appears on the icon. Chars `'!'` (index 10) and `'+'` (index 11) are
/// spares for future alert / overflow states.
const GLYPHS: [[u8; GLYPH_H]; 12] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b010, 0b010, 0b010], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
    [0b010, 0b010, 0b010, 0b000, 0b010], // '!'
    [0b000, 0b010, 0b111, 0b010, 0b000], // '+'
];

/// Build the tray icon for the current session percentage.
pub fn render_bar_icon(pct: f32, color: ThresholdColor) -> anyhow::Result<tray_icon::Icon> {
    let rgba = render_bar(pct, color);
    Ok(tray_icon::Icon::from_rgba(rgba, SIZE as u32, SIZE as u32)?)
}

/// Neutral "no data yet" icon — empty outlined bar, no digits.
pub fn render_unknown_icon() -> anyhow::Result<tray_icon::Icon> {
    let rgba = render_empty();
    Ok(tray_icon::Icon::from_rgba(rgba, SIZE as u32, SIZE as u32)?)
}

fn render_bar(pct: f32, color: ThresholdColor) -> Vec<u8> {
    let [r, g, b, _] = color.rgba();
    let fill_color = [r, g, b, 0xFF];
    let empty_color = [0x1f, 0x29, 0x3b, 0xFF];
    let border_color = [0x0a, 0x0e, 0x17, 0xFF];

    let clamped = pct.clamp(0.0, 100.0);
    let fill_end = ((clamped / 100.0) * SIZE as f32).round() as i32;

    let mut buf = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let is_border = x == 0 || x == SIZE - 1 || y == 0 || y == SIZE - 1;
            // Drop the four corners for a faint rounded feel.
            let is_corner = (x == 0 || x == SIZE - 1) && (y == 0 || y == SIZE - 1);
            if is_corner {
                continue; // leave transparent
            }
            let p = if is_border {
                border_color
            } else if x < fill_end {
                fill_color
            } else {
                empty_color
            };
            set_pixel(&mut buf, x, y, p);
        }
    }

    // Overlay the integer percentage.
    let pct_clamped = clamped.round() as i32;
    let text = if pct_clamped >= 100 {
        "!!".to_string()
    } else {
        pct_clamped.to_string()
    };
    overlay_text(&mut buf, &text);
    buf
}

fn render_empty() -> Vec<u8> {
    let outline = [0xe2, 0xe8, 0xf0, 0xFF];
    let fill = [0x1f, 0x29, 0x3b, 0xFF];
    let mut buf = vec![0u8; (SIZE * SIZE * 4) as usize];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let is_border = x == 0 || x == SIZE - 1 || y == 0 || y == SIZE - 1;
            let is_corner = (x == 0 || x == SIZE - 1) && (y == 0 || y == SIZE - 1);
            if is_corner {
                continue;
            }
            let p = if is_border { outline } else { fill };
            set_pixel(&mut buf, x, y, p);
        }
    }
    buf
}

/// Draw `text` (digits plus possibly `!` or `+`) centered over the buffer.
/// Drops a 1-px black halo first, then the white glyph on top — the halo
/// guarantees legibility against the colored fill zone.
fn overlay_text(buf: &mut [u8], text: &str) {
    let glyphs: Vec<usize> = text
        .chars()
        .filter_map(|c| match c {
            '0'..='9' => Some((c as u8 - b'0') as usize),
            '!' => Some(10),
            '+' => Some(11),
            _ => None,
        })
        .collect();
    if glyphs.is_empty() {
        return;
    }
    let glyph_w_px = GLYPH_W as i32 * SCALE;
    let glyph_h_px = GLYPH_H as i32 * SCALE;
    let gap = 1;
    let total_w = glyphs.len() as i32 * glyph_w_px + (glyphs.len() as i32 - 1) * gap;
    let origin_x = (SIZE - total_w) / 2;
    let origin_y = (SIZE - glyph_h_px) / 2;

    let halo = [0x00, 0x00, 0x00, 0xFF];
    let white = [0xFF, 0xFF, 0xFF, 0xFF];

    // Pass 1: halo (4-neighbour offsets).
    for (i, gi) in glyphs.iter().enumerate() {
        let x = origin_x + i as i32 * (glyph_w_px + gap);
        for &(dx, dy) in &[(-1, 0), (1, 0), (0, -1), (0, 1)] {
            draw_glyph(buf, x + dx, origin_y + dy, *gi, halo);
        }
    }
    // Pass 2: white on top.
    for (i, gi) in glyphs.iter().enumerate() {
        let x = origin_x + i as i32 * (glyph_w_px + gap);
        draw_glyph(buf, x, origin_y, *gi, white);
    }
}

fn draw_glyph(buf: &mut [u8], x0: i32, y0: i32, glyph_idx: usize, color: [u8; 4]) {
    let rows = GLYPHS[glyph_idx];
    for (row_i, row_bits) in rows.iter().enumerate() {
        for col_i in 0..GLYPH_W {
            let bit = (row_bits >> (GLYPH_W - 1 - col_i)) & 1;
            if bit == 0 {
                continue;
            }
            // Paint a SCALE×SCALE block.
            for sy in 0..SCALE {
                for sx in 0..SCALE {
                    let px = x0 + (col_i as i32) * SCALE + sx;
                    let py = y0 + (row_i as i32) * SCALE + sy;
                    if px >= 0 && px < SIZE && py >= 0 && py < SIZE {
                        set_pixel(buf, px, py, color);
                    }
                }
            }
        }
    }
}

#[inline]
fn set_pixel(buf: &mut [u8], x: i32, y: i32, color: [u8; 4]) {
    let idx = ((y * SIZE + x) * 4) as usize;
    buf[idx] = color[0];
    buf[idx + 1] = color[1];
    buf[idx + 2] = color[2];
    buf[idx + 3] = color[3];
}
