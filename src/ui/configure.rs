use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::app::UserConfigState;
use crate::ui::icons::Icons;
use crate::ui::theme;

const POPUP_WIDTH: u16 = 54;
const POPUP_HEIGHT: u16 = 18;

pub fn render(state: &UserConfigState, icons: &Icons, frame: &mut Frame) {
    let area = frame.area();
    if area.width < 30 || area.height < 12 {
        return;
    }

    let popup_w = POPUP_WIDTH.min(area.width.saturating_sub(4));
    let popup_h = POPUP_HEIGHT.min(area.height.saturating_sub(4));
    let popup = centered_rect(popup_w, popup_h, area);
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    // Fill background
    for y in popup.y..popup.y + popup.height {
        for x in popup.x..popup.x + popup.width {
            if in_bounds(x, y, &bounds) {
                buf[(x, y)].set_char(' ');
                buf[(x, y)].set_style(Style::default().bg(theme::BG));
            }
        }
    }

    let border_color = theme::CYAN;
    render_border(buf, &bounds, popup, border_color);
    render_title(buf, &bounds, popup, border_color);

    let left = popup.x + 3;
    let right = popup.x + popup.width.saturating_sub(3);
    let inner_w = (right - left) as usize;

    if state.loading {
        let msg = "Loading...";
        let mx = left + (inner_w.saturating_sub(msg.len())) as u16 / 2;
        let my = popup.y + popup.height / 2;
        write_str(
            buf,
            &bounds,
            mx,
            my,
            msg,
            Style::default().fg(theme::CYAN).bg(theme::BG),
        );
        return;
    }

    let label_s = Style::default().fg(theme::DIM).bg(theme::BG);
    let value_s = Style::default().fg(theme::TEXT).bg(theme::BG);
    let label_x = left + 2;
    let value_x = left + 17;

    let mut row = popup.y + 2;

    // USER ID (read-only)
    write_str(buf, &bounds, label_x, row, "USER ID", label_s);
    let id_display = truncate_str(&state.user_id, (right - value_x) as usize);
    write_str(buf, &bounds, value_x, row, &id_display, value_s);
    row += 1;

    // DEVICE ID (read-only)
    write_str(buf, &bounds, label_x, row, "DEVICE ID", label_s);
    let dev_display = truncate_str(&state.device_id, (right - value_x) as usize);
    write_str(buf, &bounds, value_x, row, &dev_display, value_s);
    row += 1;

    // HOMESERVER (read-only)
    write_str(buf, &bounds, label_x, row, "HOMESERVER", label_s);
    let hs_display = truncate_str(&state.homeserver, (right - value_x) as usize);
    write_str(buf, &bounds, value_x, row, &hs_display, value_s);
    row += 1;

    row += 1;

    // ── Field 0: DISPLAY NAME (editable) ──
    let name_selected = state.selected_field == 0;
    let name_marker_color = if name_selected {
        theme::CYAN
    } else {
        theme::DIM
    };
    let name_label_color = if name_selected {
        theme::CYAN
    } else {
        theme::TEXT
    };
    let name_marker = if name_selected {
        icons.selected
    } else {
        icons.unselected
    };

    let name_marker_s = Style::default().fg(name_marker_color).bg(theme::BG);
    let name_label_s = Style::default()
        .fg(name_label_color)
        .bg(theme::BG)
        .add_modifier(if name_selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });

    write_str(buf, &bounds, left, row, name_marker, name_marker_s);
    set_cell(
        buf,
        &bounds,
        left + 1,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );
    write_str(buf, &bounds, label_x, row, "DISPLAY NAME", name_label_s);

    let max_name_w = (right - value_x) as usize;
    if state.editing_display_name {
        let display = truncate_str(&state.display_name_buffer, max_name_w.saturating_sub(1));
        let edit_s = Style::default().fg(theme::GREEN).bg(theme::BG);
        write_str(buf, &bounds, value_x, row, &display, edit_s);
        // Cursor underscore
        let cursor_x = value_x + display.chars().count() as u16;
        let cursor_s = Style::default()
            .fg(theme::GREEN)
            .bg(theme::BG)
            .add_modifier(Modifier::SLOW_BLINK);
        set_cell(buf, &bounds, cursor_x, row, '_', cursor_s);
    } else {
        let name = state.display_name.as_deref().unwrap_or("\u{2014}");
        let name_display = truncate_str(name, max_name_w);
        let name_val_color = if name_selected {
            theme::TEXT
        } else {
            theme::DIM
        };
        let name_val_s = Style::default().fg(name_val_color).bg(theme::BG);
        write_str(buf, &bounds, value_x, row, &name_display, name_val_s);
    }
    row += 1;

    // ── Field 1: VERIFIED (actionable) ──
    let ver_selected = state.selected_field == 1;
    let ver_marker_color = if ver_selected {
        theme::CYAN
    } else {
        theme::DIM
    };
    let ver_label_color = if ver_selected {
        theme::CYAN
    } else {
        theme::TEXT
    };
    let ver_marker = if ver_selected { icons.selected } else { icons.unselected };

    let ver_marker_s = Style::default().fg(ver_marker_color).bg(theme::BG);
    let ver_label_s = Style::default()
        .fg(ver_label_color)
        .bg(theme::BG)
        .add_modifier(if ver_selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });

    write_str(buf, &bounds, left, row, ver_marker, ver_marker_s);
    set_cell(
        buf,
        &bounds,
        left + 1,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );
    write_str(buf, &bounds, label_x, row, "VERIFIED", ver_label_s);

    let (ver_text, ver_color) = if state.verified {
        ("yes", theme::GREEN)
    } else {
        ("no", Color::Rgb(200, 60, 60))
    };
    write_str(
        buf,
        &bounds,
        value_x,
        row,
        ver_text,
        Style::default().fg(ver_color).bg(theme::BG),
    );

    // Show "Enter to verify" hint when selected and not verified
    if ver_selected && !state.verified {
        let hint = "(Enter to verify)";
        let hx = value_x + ver_text.len() as u16 + 2;
        write_str(
            buf,
            &bounds,
            hx,
            row,
            hint,
            Style::default().fg(theme::DIM).bg(theme::BG),
        );
    }

    // Show saving indicator
    if state.saving {
        row += 2;
        let msg = "saving...";
        let sx = left + (inner_w.saturating_sub(msg.len())) as u16 / 2;
        write_str(
            buf,
            &bounds,
            sx,
            row,
            msg,
            Style::default()
                .fg(theme::GREEN)
                .bg(theme::BG)
                .add_modifier(Modifier::BOLD),
        );
    }

    // Hints
    let hint_row = popup.y + popup.height.saturating_sub(2);
    let hint = "j/k navigate  Enter edit  Esc close";
    let hx = left + (inner_w.saturating_sub(hint.chars().count())) as u16 / 2;
    write_str(
        buf,
        &bounds,
        hx,
        hint_row,
        hint,
        Style::default().fg(theme::DIM).bg(theme::BG),
    );
}

// ── helpers ──────────────────────────────────────────

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}…", &s[..max.saturating_sub(1)])
    } else {
        s.to_string()
    }
}

fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(w) / 2,
        area.y + area.height.saturating_sub(h) / 2,
        w.min(area.width),
        h.min(area.height),
    )
}

#[inline]
fn in_bounds(x: u16, y: u16, r: &Rect) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

#[inline]
fn set_cell(buf: &mut Buffer, bounds: &Rect, x: u16, y: u16, ch: char, style: Style) {
    if in_bounds(x, y, bounds) {
        let cell = &mut buf[(x, y)];
        cell.set_char(ch);
        cell.set_style(style);
    }
}

fn write_str(buf: &mut Buffer, bounds: &Rect, x: u16, y: u16, text: &str, style: Style) {
    for (i, ch) in text.chars().enumerate() {
        set_cell(buf, bounds, x + i as u16, y, ch, style);
    }
}

fn render_border(buf: &mut Buffer, bounds: &Rect, area: Rect, color: Color) {
    let s = Style::default().fg(color).bg(theme::BG);
    let x1 = area.x;
    let x2 = area.x + area.width - 1;
    let y1 = area.y;
    let y2 = area.y + area.height - 1;

    set_cell(buf, bounds, x1, y1, '╔', s);
    set_cell(buf, bounds, x2, y1, '╗', s);
    set_cell(buf, bounds, x1, y2, '╚', s);
    set_cell(buf, bounds, x2, y2, '╝', s);

    for x in (x1 + 1)..x2 {
        set_cell(buf, bounds, x, y1, '═', s);
        set_cell(buf, bounds, x, y2, '═', s);
    }

    for y in (y1 + 1)..y2 {
        set_cell(buf, bounds, x1, y, '║', s);
        set_cell(buf, bounds, x2, y, '║', s);
    }

    // Decorative glyph
    let gx = x2.saturating_sub(5);
    if gx > x1 {
        set_cell(buf, bounds, gx, y2, '◈', s);
    }
}

fn render_title(buf: &mut Buffer, bounds: &Rect, area: Rect, color: Color) {
    let title = "CONFIGURE";
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
