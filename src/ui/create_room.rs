use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::app::{CreateRoomState, HISTORY_VISIBILITY_OPTIONS};
use crate::ui::icons::Icons;
use crate::ui::theme;

const POPUP_WIDTH: u16 = 54;
const POPUP_HEIGHT: u16 = 22;

pub fn render(state: &CreateRoomState, icons: &Icons, frame: &mut Frame) {
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

    let label_x = left + 2;
    let value_x = left + 15;

    let mut row = popup.y + 3;

    // ── Field 0: NAME (editable) ──
    let name_selected = state.selected_field == 0;
    render_field_label(
        buf,
        &bounds,
        left,
        label_x,
        row,
        "NAME",
        name_selected,
        icons,
    );

    let max_name_w = (right - value_x) as usize;
    if state.editing_name {
        let display = truncate_str(&state.name_buffer, max_name_w.saturating_sub(1));
        let edit_s = Style::default().fg(theme::GREEN).bg(theme::BG);
        write_str(buf, &bounds, value_x, row, &display, edit_s);
        let cursor_x = value_x + display.chars().count() as u16;
        let cursor_s = Style::default()
            .fg(theme::GREEN)
            .bg(theme::BG)
            .add_modifier(Modifier::SLOW_BLINK);
        set_cell(buf, &bounds, cursor_x, row, '_', cursor_s);
    } else {
        let name = if state.name_buffer.is_empty() {
            "\u{2014}"
        } else {
            &state.name_buffer
        };
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

    // ── Field 1: TOPIC (editable) ──
    let topic_selected = state.selected_field == 1;
    render_field_label(
        buf,
        &bounds,
        left,
        label_x,
        row,
        "TOPIC",
        topic_selected,
        icons,
    );

    let max_topic_w = (right - value_x) as usize;
    if state.editing_topic {
        let display = truncate_str(&state.topic_buffer, max_topic_w.saturating_sub(1));
        let edit_s = Style::default().fg(theme::GREEN).bg(theme::BG);
        write_str(buf, &bounds, value_x, row, &display, edit_s);
        let cursor_x = value_x + display.chars().count() as u16;
        let cursor_s = Style::default()
            .fg(theme::GREEN)
            .bg(theme::BG)
            .add_modifier(Modifier::SLOW_BLINK);
        set_cell(buf, &bounds, cursor_x, row, '_', cursor_s);
    } else {
        let topic = if state.topic_buffer.is_empty() {
            "\u{2014}"
        } else {
            &state.topic_buffer
        };
        let topic_display = truncate_str(topic, max_topic_w);
        let topic_val_color = if topic_selected {
            theme::TEXT
        } else {
            theme::DIM
        };
        let topic_val_s = Style::default().fg(topic_val_color).bg(theme::BG);
        write_str(buf, &bounds, value_x, row, &topic_display, topic_val_s);
    }
    row += 1;

    // ── Field 2: HISTORY (cycle selector) ──
    let hist_selected = state.selected_field == 2;
    render_field_label(
        buf,
        &bounds,
        left,
        label_x,
        row,
        "HISTORY",
        hist_selected,
        icons,
    );

    let arrow_color = if hist_selected {
        theme::CYAN
    } else {
        theme::DIM
    };
    let vis_val_color = if hist_selected {
        theme::TEXT
    } else {
        theme::DIM
    };
    let arrow_s = Style::default().fg(arrow_color).bg(theme::BG);
    let vis_s = Style::default().fg(vis_val_color).bg(theme::BG);

    write_str(buf, &bounds, value_x, row, icons.arrow_left, arrow_s);
    set_cell(
        buf,
        &bounds,
        value_x + 1,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );

    let vis_display = &state.history_visibility;
    write_str(buf, &bounds, value_x + 2, row, vis_display, vis_s);

    let end_x = value_x + 2 + vis_display.chars().count() as u16;
    set_cell(
        buf,
        &bounds,
        end_x,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );
    write_str(buf, &bounds, end_x + 1, row, icons.arrow_right, arrow_s);
    row += 1;

    // History visibility description
    let desc = history_visibility_description(&state.history_visibility);
    let desc_s = Style::default().fg(theme::DIM).bg(theme::BG);
    write_str(buf, &bounds, value_x, row, desc, desc_s);
    row += 1;

    // ── Field 3: ENCRYPTED (toggle) ──
    let enc_selected = state.selected_field == 3;
    render_field_label(
        buf,
        &bounds,
        left,
        label_x,
        row,
        "ENCRYPTED",
        enc_selected,
        icons,
    );

    let enc_arrow_color = if enc_selected {
        theme::CYAN
    } else {
        theme::DIM
    };
    let enc_val_color = if enc_selected {
        theme::TEXT
    } else {
        theme::DIM
    };
    let enc_arrow_s = Style::default().fg(enc_arrow_color).bg(theme::BG);
    let enc_val_s = Style::default().fg(enc_val_color).bg(theme::BG);

    write_str(buf, &bounds, value_x, row, icons.arrow_left, enc_arrow_s);
    set_cell(
        buf,
        &bounds,
        value_x + 1,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );

    let enc_display = &state.encrypted;
    write_str(buf, &bounds, value_x + 2, row, enc_display, enc_val_s);

    let enc_end_x = value_x + 2 + enc_display.chars().count() as u16;
    set_cell(
        buf,
        &bounds,
        enc_end_x,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );
    write_str(
        buf,
        &bounds,
        enc_end_x + 1,
        row,
        icons.arrow_right,
        enc_arrow_s,
    );
    row += 2;

    // ── Field 4: CREATE button ──
    let btn_selected = state.selected_field == 4;
    let btn_label = if state.creating {
        "  creating...  "
    } else {
        "  [ CREATE ]  "
    };
    let btn_x = left + (inner_w.saturating_sub(btn_label.len())) as u16 / 2;
    let btn_style = if state.creating {
        Style::default()
            .fg(theme::CYAN)
            .bg(theme::BG)
            .add_modifier(Modifier::BOLD)
    } else if btn_selected {
        Style::default()
            .fg(theme::GREEN)
            .bg(theme::BG)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme::DIM).bg(theme::BG)
    };
    write_str(buf, &bounds, btn_x, row, btn_label, btn_style);

    // Show valid options hint
    let opts_row = popup.y + popup.height.saturating_sub(4);
    let opts: String = if state.selected_field == 3 {
        "no | yes".to_string()
    } else {
        HISTORY_VISIBILITY_OPTIONS.join(" | ")
    };
    let opts_x = left + (inner_w.saturating_sub(opts.len())) as u16 / 2;
    write_str(
        buf,
        &bounds,
        opts_x,
        opts_row,
        &opts,
        Style::default().fg(Color::Rgb(60, 60, 80)).bg(theme::BG),
    );

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

fn render_field_label(
    buf: &mut Buffer,
    bounds: &Rect,
    left: u16,
    label_x: u16,
    row: u16,
    label: &str,
    selected: bool,
    icons: &Icons,
) {
    let marker_color = if selected { theme::CYAN } else { theme::DIM };
    let label_color = if selected { theme::CYAN } else { theme::TEXT };
    let marker = if selected {
        icons.selected
    } else {
        icons.unselected
    };

    let marker_s = Style::default().fg(marker_color).bg(theme::BG);
    let label_s = Style::default()
        .fg(label_color)
        .bg(theme::BG)
        .add_modifier(if selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });

    write_str(buf, bounds, left, row, marker, marker_s);
    set_cell(
        buf,
        bounds,
        left + 1,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );
    write_str(buf, bounds, label_x, row, label, label_s);
}

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

    let gx = x2.saturating_sub(5);
    if gx > x1 {
        set_cell(buf, bounds, gx, y2, '◈', s);
    }
}

fn history_visibility_description(value: &str) -> &'static str {
    match value {
        "shared" => "All members see full history",
        "invited" => "See history from when invited",
        "joined" => "See history from when joined",
        "world_readable" => "Anyone can read full history",
        _ => "",
    }
}

fn render_title(buf: &mut Buffer, bounds: &Rect, area: Rect, color: Color) {
    let title = "ROOM CREATE";
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
