use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::app::App;
use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhichKeyCategory {
    Room,
    Call,
    Security,
    Effects,
    User,
}

struct Entry {
    key: char,
    label: &'static str,
    available: fn(&App) -> bool,
}

fn always(_: &App) -> bool {
    true
}
fn room_selected(app: &App) -> bool {
    app.messages.current_room_id.is_some()
}
fn no_active_call(app: &App) -> bool {
    app.call_info.is_none()
}
fn incoming_call(app: &App) -> bool {
    app.incoming_call_room.is_some()
}
fn active_call(app: &App) -> bool {
    app.call_info.is_some()
}

fn entries(cat: WhichKeyCategory) -> &'static [Entry] {
    match cat {
        WhichKeyCategory::Room => &[
            Entry { key: 'j', label: "Join room", available: always },
            Entry { key: 'l', label: "Leave room", available: room_selected },
            Entry { key: 'c', label: "Create room", available: always },
            Entry { key: 'e', label: "Edit room", available: room_selected },
            Entry { key: 'd', label: "DM user", available: always },
        ],
        WhichKeyCategory::Call => &[
            Entry { key: 'c', label: "Start call", available: |app| room_selected(app) && no_active_call(app) },
            Entry { key: 'a', label: "Answer", available: incoming_call },
            Entry { key: 'd', label: "Decline", available: incoming_call },
            Entry { key: 'h', label: "Hangup", available: active_call },
        ],
        WhichKeyCategory::Security => &[
            Entry { key: 'v', label: "Verify", available: always },
            Entry { key: 'r', label: "Recovery", available: always },
        ],
        WhichKeyCategory::Effects => &[
            Entry { key: 'r', label: "Rain toggle", available: always },
            Entry { key: 'g', label: "Glitch toggle", available: always },
        ],
        WhichKeyCategory::User => &[
            Entry { key: 'p', label: "Profile", available: always },
            Entry { key: 'a', label: "Audio", available: always },
        ],
    }
}

fn category_title(cat: WhichKeyCategory) -> &'static str {
    match cat {
        WhichKeyCategory::Room => "ROOM",
        WhichKeyCategory::Call => "CALL",
        WhichKeyCategory::Security => "SECURITY",
        WhichKeyCategory::Effects => "EFFECTS",
        WhichKeyCategory::User => "USER",
    }
}

// ── Rendering ──────────────────────────────────────────

pub fn render(which_key: Option<WhichKeyCategory>, app: &App, frame: &mut Frame) {
    match which_key {
        None => render_root(frame),
        Some(cat) => render_category(cat, app, frame),
    }
}

fn render_root(frame: &mut Frame) {
    let area = frame.area();
    if area.width < 30 || area.height < 10 {
        return;
    }

    let popup_h: u16 = 10;
    let popup = bottom_rect(popup_h, area);
    let popup_w = popup.width;
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    fill_bg(buf, &bounds, popup);
    render_border(buf, &bounds, popup, theme::CYAN);
    render_title_str(buf, &bounds, popup, theme::CYAN, "LEADER");

    let left = popup.x + 3;
    let col2 = popup.x + popup_w / 2 + 1;
    let mut y = popup.y + 2;

    let key_style = Style::default()
        .fg(theme::MAGENTA)
        .bg(theme::BG)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(theme::TEXT).bg(theme::BG);

    // Categories: two columns
    // Row 1: r Room, c Call
    render_entry(buf, &bounds, left, y, 'r', "Room", key_style, label_style);
    render_entry(buf, &bounds, col2, y, 'c', "Call", key_style, label_style);
    y += 1;

    // Row 2: s Security, e Effects
    render_entry(buf, &bounds, left, y, 's', "Security", key_style, label_style);
    render_entry(buf, &bounds, col2, y, 'e', "Effects", key_style, label_style);
    y += 1;

    // Row 3: u User
    render_entry(buf, &bounds, left, y, 'u', "User", key_style, label_style);
    y += 2;

    // Actions row: q Quit, l Logout
    render_entry(buf, &bounds, left, y, 'q', "Quit", key_style, label_style);
    render_entry(buf, &bounds, col2, y, 'l', "Logout", key_style, label_style);

    // Hint
    let hint = "Esc close";
    let hx = popup.x + (popup_w.saturating_sub(hint.len() as u16)) / 2;
    write_str(
        buf,
        &bounds,
        hx,
        popup.y + popup_h - 2,
        hint,
        Style::default().fg(theme::DIM).bg(theme::BG),
    );
}

fn render_category(cat: WhichKeyCategory, app: &App, frame: &mut Frame) {
    let area = frame.area();
    if area.width < 30 || area.height < 10 {
        return;
    }

    let items = entries(cat);
    let rows = (items.len() as u16 + 1) / 2; // ceil div
    let popup_h: u16 = rows + 5; // border top + padding + rows + padding + hint + border bottom
    let popup = bottom_rect(popup_h, area);
    let popup_w = popup.width;
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    fill_bg(buf, &bounds, popup);
    render_border(buf, &bounds, popup, theme::CYAN);

    let title = format!("LEADER \u{203a} {}", category_title(cat));
    render_title_str(buf, &bounds, popup, theme::CYAN, &title);

    let left = popup.x + 3;
    let col2 = popup.x + popup_w / 2 + 1;

    let key_style = Style::default()
        .fg(theme::MAGENTA)
        .bg(theme::BG)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(theme::TEXT).bg(theme::BG);
    let dim_key_style = Style::default()
        .fg(theme::DIM)
        .bg(theme::BG)
        .add_modifier(Modifier::BOLD);
    let dim_label_style = Style::default().fg(theme::DIM).bg(theme::BG);

    let y_start = popup.y + 2;
    for (i, entry) in items.iter().enumerate() {
        let col = if i % 2 == 0 { left } else { col2 };
        let row = y_start + (i / 2) as u16;
        let avail = (entry.available)(app);
        let (ks, ls) = if avail {
            (key_style, label_style)
        } else {
            (dim_key_style, dim_label_style)
        };
        render_entry(buf, &bounds, col, row, entry.key, entry.label, ks, ls);
    }

    // Hint
    let hint = "Esc close \u{00b7} Backspace back";
    let hx = popup.x + (popup_w.saturating_sub(hint.len() as u16)) / 2;
    write_str(
        buf,
        &bounds,
        hx,
        popup.y + popup_h - 2,
        hint,
        Style::default().fg(theme::DIM).bg(theme::BG),
    );
}

fn render_entry(
    buf: &mut Buffer,
    bounds: &Rect,
    x: u16,
    y: u16,
    key: char,
    label: &str,
    key_style: Style,
    label_style: Style,
) {
    set_cell(buf, bounds, x, y, key, key_style);
    write_str(buf, bounds, x + 4, y, label, label_style);
}

// ── Helpers (same as recovery_modal) ──────────────────

fn bottom_rect(h: u16, area: Rect) -> Rect {
    let h = h.min(area.height.saturating_sub(1));
    Rect::new(
        area.x,
        area.y + area.height.saturating_sub(1).saturating_sub(h),
        area.width,
        h,
    )
}

fn fill_bg(buf: &mut Buffer, bounds: &Rect, popup: Rect) {
    for y in popup.y..popup.y + popup.height {
        for x in popup.x..popup.x + popup.width {
            if in_bounds(x, y, bounds) {
                buf[(x, y)].set_char(' ');
                buf[(x, y)].set_style(Style::default().bg(theme::BG));
            }
        }
    }
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

    set_cell(buf, bounds, x1, y1, '\u{2554}', s);
    set_cell(buf, bounds, x2, y1, '\u{2557}', s);
    set_cell(buf, bounds, x1, y2, '\u{255a}', s);
    set_cell(buf, bounds, x2, y2, '\u{255d}', s);

    for x in (x1 + 1)..x2 {
        set_cell(buf, bounds, x, y1, '\u{2550}', s);
        set_cell(buf, bounds, x, y2, '\u{2550}', s);
    }

    for y in (y1 + 1)..y2 {
        set_cell(buf, bounds, x1, y, '\u{2551}', s);
        set_cell(buf, bounds, x2, y, '\u{2551}', s);
    }

    let gx = x2.saturating_sub(5);
    if gx > x1 {
        set_cell(buf, bounds, gx, y2, '\u{25c8}', s);
    }
}

fn render_title_str(buf: &mut Buffer, bounds: &Rect, area: Rect, color: Color, title: &str) {
    let border_s = Style::default().fg(color).bg(theme::BG);
    let title_s = border_s.add_modifier(Modifier::BOLD);

    let bracket_l = area.x + 3;
    let title_start = bracket_l + 2;

    set_cell(buf, bounds, bracket_l, area.y, '\u{2561}', border_s);
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
        set_cell(buf, bounds, bracket_r, area.y, '\u{255e}', border_s);
    }
}
