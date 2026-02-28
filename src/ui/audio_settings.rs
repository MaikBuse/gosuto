use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::app::AudioSettingsState;
use crate::ui::theme;

const POPUP_WIDTH: u16 = 54;
const POPUP_HEIGHT: u16 = 20;

const BAR_WIDTH: usize = 20;

pub fn render(state: &AudioSettingsState, frame: &mut Frame) {
    let area = frame.area();
    if area.width < 30 || area.height < 14 {
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

    let visible = state.visible_fields();
    let left = popup.x + 3;
    let right = popup.x + popup.width.saturating_sub(3);
    let inner_w = (right - left) as usize;

    let mut row = popup.y + 2;

    for (vis_idx, &field_id) in visible.iter().enumerate() {
        let selected = vis_idx == state.selected_field;
        render_field(buf, left, right, row, field_id, state, selected);
        row += 1;

        // Add a blank row after output device and output volume for visual grouping
        if field_id == 1 || field_id == 3 {
            row += 1;
        }
    }

    // Mic level meter
    let meter_row = popup.y + popup.height.saturating_sub(4);
    render_mic_meter(buf, &bounds, left, right, meter_row, state.mic_level);

    // Hints
    let hint_row = popup.y + popup.height.saturating_sub(2);
    let hint = "j/k navigate  h/l adjust  Esc close";
    let hx = left + ((inner_w).saturating_sub(hint.chars().count())) as u16 / 2;
    write_str(
        buf,
        &bounds,
        hx,
        hint_row,
        hint,
        Style::default().fg(theme::DIM).bg(theme::BG),
    );
}

fn render_field(
    buf: &mut Buffer,
    left: u16,
    right: u16,
    row: u16,
    field_id: usize,
    state: &AudioSettingsState,
    selected: bool,
) {
    let bounds = *buf.area();
    let marker_color = if selected { theme::CYAN } else { theme::DIM };
    let label_color = if selected { theme::CYAN } else { theme::TEXT };
    let marker = if selected { '◈' } else { '◇' };

    let marker_s = Style::default().fg(marker_color).bg(theme::BG);
    let label_s = Style::default()
        .fg(label_color)
        .bg(theme::BG)
        .add_modifier(if selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });

    set_cell(buf, &bounds, left, row, marker, marker_s);
    set_cell(
        buf,
        &bounds,
        left + 1,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );

    let label_x = left + 2;
    let value_x = left + 19;

    match field_id {
        0 => {
            write_str(buf, &bounds, label_x, row, "INPUT DEVICE", label_s);
            let name = state
                .input_devices
                .get(state.input_device_idx)
                .map(|s| s.as_str())
                .unwrap_or("Default");
            render_device_selector(buf, &bounds, value_x, right, row, name, selected);
        }
        1 => {
            write_str(buf, &bounds, label_x, row, "OUTPUT DEVICE", label_s);
            let name = state
                .output_devices
                .get(state.output_device_idx)
                .map(|s| s.as_str())
                .unwrap_or("Default");
            render_device_selector(buf, &bounds, value_x, right, row, name, selected);
        }
        2 => {
            write_str(buf, &bounds, label_x, row, "INPUT VOLUME", label_s);
            render_volume_bar(buf, &bounds, value_x, row, state.input_volume, selected);
        }
        3 => {
            write_str(buf, &bounds, label_x, row, "OUTPUT VOLUME", label_s);
            render_volume_bar(buf, &bounds, value_x, row, state.output_volume, selected);
        }
        4 => {
            write_str(buf, &bounds, label_x, row, "VOICE ACTIVITY", label_s);
            render_toggle(buf, &bounds, value_x, row, state.voice_activity, selected);
        }
        5 => {
            write_str(buf, &bounds, label_x, row, "SENSITIVITY", label_s);
            render_volume_bar(buf, &bounds, value_x, row, state.sensitivity, selected);
        }
        6 => {
            write_str(buf, &bounds, label_x, row, "PUSH TO TALK", label_s);
            render_toggle(buf, &bounds, value_x, row, state.push_to_talk, selected);
        }
        7 => {
            write_str(buf, &bounds, label_x, row, "PTT KEY", label_s);
            if state.capturing_ptt_key {
                let s = Style::default()
                    .fg(theme::GREEN)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD);
                write_str(buf, &bounds, value_x, row, "press key...", s);
            } else {
                let key_name = state.push_to_talk_key.as_deref().unwrap_or("not set");
                let s = if selected {
                    Style::default().fg(theme::CYAN).bg(theme::BG)
                } else {
                    Style::default().fg(theme::DIM).bg(theme::BG)
                };
                let display = format!("[{}]", key_name);
                write_str(buf, &bounds, value_x, row, &display, s);
            }
        }
        _ => {}
    }
}

fn render_device_selector(
    buf: &mut Buffer,
    bounds: &Rect,
    x: u16,
    right: u16,
    row: u16,
    name: &str,
    selected: bool,
) {
    let arrow_color = if selected { theme::CYAN } else { theme::DIM };
    let name_color = if selected { theme::TEXT } else { theme::DIM };

    let arrow_s = Style::default().fg(arrow_color).bg(theme::BG);
    let name_s = Style::default().fg(name_color).bg(theme::BG);

    set_cell(buf, bounds, x, row, '◂', arrow_s);
    set_cell(buf, bounds, x + 1, row, ' ', Style::default().bg(theme::BG));

    // Truncate name to fit
    let max_name_w = (right.saturating_sub(x + 4)) as usize;
    let display: String = if name.len() > max_name_w {
        format!("{}…", &name[..max_name_w.saturating_sub(1)])
    } else {
        name.to_string()
    };
    write_str(buf, bounds, x + 2, row, &display, name_s);

    let end_x = x + 2 + display.chars().count() as u16;
    set_cell(buf, bounds, end_x, row, ' ', Style::default().bg(theme::BG));
    set_cell(buf, bounds, end_x + 1, row, '▸', arrow_s);
}

fn render_volume_bar(
    buf: &mut Buffer,
    bounds: &Rect,
    x: u16,
    row: u16,
    value: f32,
    selected: bool,
) {
    let filled = (value * BAR_WIDTH as f32).round() as usize;
    let fill_color = if selected { theme::CYAN } else { theme::DIM };
    let empty_color = Color::Rgb(40, 40, 50);
    let pct = format!("{:>3}%", (value * 100.0).round() as u32);

    set_cell(
        buf,
        bounds,
        x,
        row,
        '[',
        Style::default().fg(theme::DIM).bg(theme::BG),
    );

    for i in 0..BAR_WIDTH {
        let ch = if i < filled { '█' } else { '░' };
        let color = if i < filled { fill_color } else { empty_color };
        set_cell(
            buf,
            bounds,
            x + 1 + i as u16,
            row,
            ch,
            Style::default().fg(color).bg(theme::BG),
        );
    }

    set_cell(
        buf,
        bounds,
        x + 1 + BAR_WIDTH as u16,
        row,
        ']',
        Style::default().fg(theme::DIM).bg(theme::BG),
    );

    write_str(
        buf,
        bounds,
        x + 2 + BAR_WIDTH as u16,
        row,
        &pct,
        Style::default().fg(fill_color).bg(theme::BG),
    );
}

fn render_toggle(buf: &mut Buffer, bounds: &Rect, x: u16, row: u16, on: bool, selected: bool) {
    let (text, color) = if on {
        ("ON", theme::GREEN)
    } else {
        ("OFF", theme::RED)
    };
    let s = Style::default()
        .fg(if selected { color } else { theme::DIM })
        .bg(theme::BG)
        .add_modifier(Modifier::BOLD);
    write_str(buf, bounds, x, row, text, s);
}

fn render_mic_meter(buf: &mut Buffer, bounds: &Rect, left: u16, right: u16, row: u16, level: f32) {
    let label = "MIC ";
    let label_s = Style::default().fg(theme::DIM).bg(theme::BG);
    write_str(buf, bounds, left, row, label, label_s);

    let bar_start = left + label.len() as u16;
    let bar_width = (right.saturating_sub(bar_start)) as usize;
    let filled = (level.clamp(0.0, 1.0) * bar_width as f32).round() as usize;

    for i in 0..bar_width {
        let ch = if i < filled { '▓' } else { '░' };
        let color = if i < filled {
            if (i as f32 / bar_width as f32) > 0.8 {
                theme::RED
            } else {
                theme::GREEN
            }
        } else {
            Color::Rgb(30, 30, 40)
        };
        set_cell(
            buf,
            bounds,
            bar_start + i as u16,
            row,
            ch,
            Style::default().fg(color).bg(theme::BG),
        );
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
    let title = "AUDIO CONFIGURATION";
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

// ── helpers ──────────────────────────────────────────

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
