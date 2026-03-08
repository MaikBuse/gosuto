use ratatui::Frame;
use ratatui::style::{Modifier, Style};

use crate::app::App;
use crate::ui::popup;
use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WhichKeyCategory {
    Room,
    Call,
    Effects,
    User,
    Security,
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
            Entry {
                key: 'j',
                label: "Join room",
                available: always,
            },
            Entry {
                key: 'l',
                label: "Leave room",
                available: room_selected,
            },
            Entry {
                key: 'c',
                label: "Create room",
                available: always,
            },
            Entry {
                key: 'e',
                label: "Edit room",
                available: room_selected,
            },
            Entry {
                key: 'd',
                label: "DM user",
                available: always,
            },
        ],
        WhichKeyCategory::Call => &[
            Entry {
                key: 'c',
                label: "Start call",
                available: |app| room_selected(app) && no_active_call(app),
            },
            Entry {
                key: 'a',
                label: "Answer",
                available: incoming_call,
            },
            Entry {
                key: 'd',
                label: "Decline",
                available: incoming_call,
            },
            Entry {
                key: 'h',
                label: "Hangup",
                available: active_call,
            },
        ],
        WhichKeyCategory::Effects => &[
            Entry {
                key: 'r',
                label: "Rain toggle",
                available: always,
            },
            Entry {
                key: 'g',
                label: "Glitch toggle",
                available: always,
            },
        ],
        WhichKeyCategory::User => &[
            Entry {
                key: 'p',
                label: "Profile",
                available: always,
            },
            Entry {
                key: 'a',
                label: "Audio",
                available: always,
            },
        ],
        WhichKeyCategory::Security => &[
            Entry {
                key: 'r',
                label: "Recovery",
                available: always,
            },
            Entry {
                key: 'v',
                label: "Verify",
                available: always,
            },
        ],
    }
}

fn category_title(cat: WhichKeyCategory) -> &'static str {
    match cat {
        WhichKeyCategory::Room => "ROOM",
        WhichKeyCategory::Call => "CALL",
        WhichKeyCategory::Effects => "EFFECTS",
        WhichKeyCategory::User => "USER",
        WhichKeyCategory::Security => "SECURITY",
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
    let popup_area = bottom_rect(popup_h, area);
    let popup_w = popup_area.width;
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    popup::render_popup_chrome(buf, &bounds, popup_area, "LEADER");

    let left = popup_area.x + 3;
    let col2 = popup_area.x + popup_w / 2 + 1;
    let mut y = popup_area.y + 2;

    let key_style = Style::default()
        .fg(theme::MAGENTA)
        .bg(theme::BG)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(theme::TEXT).bg(theme::BG);

    // Categories: two columns
    // Row 1: r Room, c Call
    popup::set_cell(buf, &bounds, left, y, 'r', key_style);
    popup::write_str(buf, &bounds, left + 4, y, "Room", label_style);
    popup::set_cell(buf, &bounds, col2, y, 'c', key_style);
    popup::write_str(buf, &bounds, col2 + 4, y, "Call", label_style);
    y += 1;

    // Row 2: s Security, e Effects
    popup::set_cell(buf, &bounds, left, y, 's', key_style);
    popup::write_str(buf, &bounds, left + 4, y, "Security", label_style);
    popup::set_cell(buf, &bounds, col2, y, 'e', key_style);
    popup::write_str(buf, &bounds, col2 + 4, y, "Effects", label_style);
    y += 1;

    // Row 3: u User
    popup::set_cell(buf, &bounds, left, y, 'u', key_style);
    popup::write_str(buf, &bounds, left + 4, y, "User", label_style);
    y += 1;
    y += 1;

    // Actions row: q Quit, l Logout
    popup::set_cell(buf, &bounds, left, y, 'q', key_style);
    popup::write_str(buf, &bounds, left + 4, y, "Quit", label_style);
    popup::set_cell(buf, &bounds, col2, y, 'l', key_style);
    popup::write_str(buf, &bounds, col2 + 4, y, "Logout", label_style);

    // Hint
    popup::render_hint(buf, &bounds, popup_area, "Esc close");
}

fn render_category(cat: WhichKeyCategory, app: &App, frame: &mut Frame) {
    let area = frame.area();
    if area.width < 30 || area.height < 10 {
        return;
    }

    let items = entries(cat);
    let rows = (items.len() as u16).div_ceil(2);
    let popup_h: u16 = rows + 5; // border top + padding + rows + padding + hint + border bottom
    let popup_area = bottom_rect(popup_h, area);
    let popup_w = popup_area.width;
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    popup::render_popup_chrome(buf, &bounds, popup_area, "");

    let title = format!("LEADER › {}", category_title(cat));
    popup::render_title(buf, &bounds, popup_area, theme::CYAN, &title);

    let left = popup_area.x + 3;
    let col2 = popup_area.x + popup_w / 2 + 1;

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

    let y_start = popup_area.y + 2;
    for (i, entry) in items.iter().enumerate() {
        let col = if i % 2 == 0 { left } else { col2 };
        let row = y_start + (i / 2) as u16;
        let avail = (entry.available)(app);
        let (ks, ls) = if avail {
            (key_style, label_style)
        } else {
            (dim_key_style, dim_label_style)
        };
        popup::set_cell(buf, &bounds, col, row, entry.key, ks);
        popup::write_str(buf, &bounds, col + 4, row, entry.label, ls);
    }

    // Hint
    popup::render_hint(buf, &bounds, popup_area, "Esc close · Backspace back");
}

// ── Helpers ───────────────────────────────────────────

fn bottom_rect(h: u16, area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    let h = h.min(area.height.saturating_sub(1));
    ratatui::layout::Rect::new(
        area.x,
        area.y + area.height.saturating_sub(1).saturating_sub(h),
        area.width,
        h,
    )
}
