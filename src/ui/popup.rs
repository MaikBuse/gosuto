use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::ui::{gradient, theme};

pub fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(w) / 2,
        area.y + area.height.saturating_sub(h) / 2,
        w.min(area.width),
        h.min(area.height),
    )
}

#[inline]
pub fn in_bounds(x: u16, y: u16, r: &Rect) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

#[inline]
pub fn set_cell(buf: &mut Buffer, bounds: &Rect, x: u16, y: u16, ch: char, style: Style) {
    if in_bounds(x, y, bounds) {
        let cell = &mut buf[(x, y)];
        cell.set_char(ch);
        cell.set_style(style);
        cell.skip = false;
    }
}

pub fn write_str(buf: &mut Buffer, bounds: &Rect, x: u16, y: u16, text: &str, style: Style) {
    for (i, ch) in text.chars().enumerate() {
        set_cell(buf, bounds, x + i as u16, y, ch, style);
    }
}

pub fn fill_bg(buf: &mut Buffer, bounds: &Rect, popup: Rect) {
    for y in popup.y..popup.y + popup.height {
        for x in popup.x..popup.x + popup.width {
            if in_bounds(x, y, bounds) {
                let cell = &mut buf[(x, y)];
                cell.set_char(' ');
                cell.set_style(Style::default().bg(theme::BG));
                cell.skip = false;
            }
        }
    }
}

pub fn truncate_str(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max.saturating_sub(1)])
    } else {
        s.to_string()
    }
}

/// Render popup border with a perimeter gradient (cyan → magenta).
/// Uses double-line box characters: ╔ ╗ ╚ ╝ ═ ║
pub fn render_gradient_border(buf: &mut Buffer, bounds: &Rect, area: Rect, phase: f32) {
    let x1 = area.x;
    let x2 = area.x + area.width - 1;
    let y1 = area.y;
    let y2 = area.y + area.height - 1;

    // Collect perimeter positions clockwise with their characters
    let mut perimeter: Vec<(u16, u16, char)> = Vec::new();

    // Top edge
    perimeter.push((x1, y1, '╔'));
    for x in (x1 + 1)..x2 {
        perimeter.push((x, y1, '═'));
    }
    perimeter.push((x2, y1, '╗'));

    // Right edge (skip corner)
    for y in (y1 + 1)..y2 {
        perimeter.push((x2, y, '║'));
    }

    // Bottom edge (reversed)
    perimeter.push((x2, y2, '╝'));
    for x in ((x1 + 1)..x2).rev() {
        perimeter.push((x, y2, '═'));
    }
    perimeter.push((x1, y2, '╚'));

    // Left edge (reversed, skip corners)
    for y in ((y1 + 1)..y2).rev() {
        perimeter.push((x1, y, '║'));
    }

    let total = perimeter.len();
    for (i, &(x, y, ch)) in perimeter.iter().enumerate() {
        let angle = (i as f32 / total as f32) * std::f32::consts::TAU + phase;
        let t = (1.0 - angle.cos()) / 2.0;
        let color =
            gradient::lerp_color(theme::GRADIENT_BORDER_START, theme::GRADIENT_BORDER_END, t);
        set_cell(
            buf,
            bounds,
            x,
            y,
            ch,
            Style::default().fg(color).bg(theme::BG),
        );
    }

    // ◈ accent in MAGENTA
    let gx = x2.saturating_sub(5);
    if gx > x1 {
        set_cell(
            buf,
            bounds,
            gx,
            y2,
            '◈',
            Style::default().fg(theme::MAGENTA).bg(theme::BG),
        );
    }
}

pub fn render_title(buf: &mut Buffer, bounds: &Rect, area: Rect, color: Color, title: &str) {
    let border_s = Style::default().fg(color).bg(theme::BG);
    let title_s = border_s.add_modifier(Modifier::BOLD);

    let bracket_l = area.x + 3;
    let title_start = bracket_l + 2;

    set_cell(buf, bounds, bracket_l, area.y, '╡', border_s);
    set_cell(buf, bounds, bracket_l + 1, area.y, ' ', border_s);

    for (i, ch) in title.chars().enumerate() {
        let x = title_start + i as u16;
        if x >= area.x + area.width - 1 {
            break;
        }
        set_cell(buf, bounds, x, area.y, ch, title_s);
    }

    let bracket_r_space = title_start + title.len() as u16;
    let bracket_r = bracket_r_space + 1;
    set_cell(buf, bounds, bracket_r_space, area.y, ' ', border_s);
    if bracket_r < area.x + area.width - 1 {
        set_cell(buf, bounds, bracket_r, area.y, '╞', border_s);
    }
}

pub fn render_hint(buf: &mut Buffer, bounds: &Rect, popup: Rect, hint: &str) {
    let hint_row = popup.y + popup.height.saturating_sub(2);
    let inner_w = popup.width.saturating_sub(6) as usize;
    let left = popup.x + 3;
    let hx = left + (inner_w.saturating_sub(hint.chars().count())) as u16 / 2;
    write_str(
        buf,
        bounds,
        hx,
        hint_row,
        hint,
        Style::default().fg(theme::DIM).bg(theme::BG),
    );
}

pub fn render_popup_chrome(buf: &mut Buffer, bounds: &Rect, popup: Rect, title: &str, phase: f32) {
    fill_bg(buf, bounds, popup);
    render_gradient_border(buf, bounds, popup, phase);
    render_title(buf, bounds, popup, theme::CYAN, title);
}

pub fn history_visibility_description(value: &str) -> &'static str {
    match value {
        "shared" => "All members see full history",
        "invited" => "See history from when invited",
        "joined" => "See history from when joined",
        "world_readable" => "Anyone can read full history",
        _ => "",
    }
}
