use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::app::AudioSettingsState;
use crate::ui::icons::Icons;
use crate::ui::popup;
use crate::ui::theme;

const POPUP_WIDTH: u16 = 54;
const POPUP_HEIGHT: u16 = 20;

const BAR_WIDTH: usize = 20;

pub fn render(state: &AudioSettingsState, icons: &Icons, frame: &mut Frame) {
    let area = frame.area();
    if area.width < 30 || area.height < 14 {
        return;
    }

    let popup_w = POPUP_WIDTH.min(area.width.saturating_sub(4));
    let popup_h = POPUP_HEIGHT.min(area.height.saturating_sub(4));
    let popup_area = popup::centered_rect(popup_w, popup_h, area);
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    popup::render_popup_chrome(buf, &bounds, popup_area, "AUDIO CONFIGURATION");

    let visible = state.visible_fields();
    let left = popup_area.x + 3;
    let right = popup_area.x + popup_area.width.saturating_sub(3);

    let mut row = popup_area.y + 2;

    for (vis_idx, &field_id) in visible.iter().enumerate() {
        let selected = vis_idx == state.selected_field;
        render_field(buf, (left, right), row, field_id, state, selected, icons);
        row += 1;

        // Add a blank row after output device and output volume for visual grouping
        if field_id == 1 || field_id == 3 {
            row += 1;
        }
    }

    // Mic level meter
    let meter_row = popup_area.y + popup_area.height.saturating_sub(4);
    render_mic_meter(buf, &bounds, left, right, meter_row, state.mic_level);

    // Hints
    popup::render_hint(
        buf,
        &bounds,
        popup_area,
        "j/k navigate  h/l adjust  Esc close",
    );
}

fn render_field(
    buf: &mut Buffer,
    cols: (u16, u16),
    row: u16,
    field_id: usize,
    state: &AudioSettingsState,
    selected: bool,
    icons: &Icons,
) {
    let (left, right) = cols;
    let bounds = *buf.area();
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

    popup::write_str(buf, &bounds, left, row, marker, marker_s);
    popup::set_cell(
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
            popup::write_str(buf, &bounds, label_x, row, "INPUT DEVICE", label_s);
            let name = state
                .input_devices
                .get(state.input_device_idx)
                .map(|s| s.as_str())
                .unwrap_or("Default");
            render_device_selector(buf, value_x, right, row, name, selected, icons);
        }
        1 => {
            popup::write_str(buf, &bounds, label_x, row, "OUTPUT DEVICE", label_s);
            let name = state
                .output_devices
                .get(state.output_device_idx)
                .map(|s| s.as_str())
                .unwrap_or("Default");
            render_device_selector(buf, value_x, right, row, name, selected, icons);
        }
        2 => {
            popup::write_str(buf, &bounds, label_x, row, "INPUT VOLUME", label_s);
            render_volume_bar(buf, &bounds, value_x, row, state.input_volume, selected);
        }
        3 => {
            popup::write_str(buf, &bounds, label_x, row, "OUTPUT VOLUME", label_s);
            render_volume_bar(buf, &bounds, value_x, row, state.output_volume, selected);
        }
        4 => {
            popup::write_str(buf, &bounds, label_x, row, "VOICE ACTIVITY", label_s);
            render_toggle(buf, &bounds, value_x, row, state.voice_activity, selected);
        }
        5 => {
            popup::write_str(buf, &bounds, label_x, row, "SENSITIVITY", label_s);
            render_volume_bar(buf, &bounds, value_x, row, state.sensitivity, selected);
        }
        6 => {
            popup::write_str(buf, &bounds, label_x, row, "PUSH TO TALK", label_s);
            render_toggle(buf, &bounds, value_x, row, state.push_to_talk, selected);
        }
        7 => {
            popup::write_str(buf, &bounds, label_x, row, "PTT KEY", label_s);
            if state.capturing_ptt_key {
                let s = Style::default()
                    .fg(theme::GREEN)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD);
                popup::write_str(buf, &bounds, value_x, row, "press key...", s);
            } else if let Some(ref err) = state.ptt_error {
                let s = Style::default()
                    .fg(theme::RED)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD);
                let max_w = (right.saturating_sub(value_x)) as usize;
                let display: &str = if err.len() > max_w {
                    &err[..max_w]
                } else {
                    err
                };
                popup::write_str(buf, &bounds, value_x, row, display, s);
            } else {
                let key_name = state.push_to_talk_key.as_deref().unwrap_or("not set");
                let s = if selected {
                    Style::default().fg(theme::CYAN).bg(theme::BG)
                } else {
                    Style::default().fg(theme::DIM).bg(theme::BG)
                };
                let display = format!("[{}]", key_name);
                popup::write_str(buf, &bounds, value_x, row, &display, s);
            }
        }
        _ => {}
    }
}

fn render_device_selector(
    buf: &mut Buffer,
    x: u16,
    right: u16,
    row: u16,
    name: &str,
    selected: bool,
    icons: &Icons,
) {
    let bounds = *buf.area();
    let arrow_color = if selected { theme::CYAN } else { theme::DIM };
    let name_color = if selected { theme::TEXT } else { theme::DIM };

    let arrow_s = Style::default().fg(arrow_color).bg(theme::BG);
    let name_s = Style::default().fg(name_color).bg(theme::BG);

    popup::write_str(buf, &bounds, x, row, icons.arrow_left, arrow_s);
    popup::set_cell(
        buf,
        &bounds,
        x + 1,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );

    // Truncate name to fit
    let max_name_w = (right.saturating_sub(x + 4)) as usize;
    let display: String = if name.len() > max_name_w {
        format!("{}…", &name[..max_name_w.saturating_sub(1)])
    } else {
        name.to_string()
    };
    popup::write_str(buf, &bounds, x + 2, row, &display, name_s);

    let end_x = x + 2 + display.chars().count() as u16;
    popup::set_cell(
        buf,
        &bounds,
        end_x,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );
    popup::write_str(buf, &bounds, end_x + 1, row, icons.arrow_right, arrow_s);
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

    popup::set_cell(
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
        popup::set_cell(
            buf,
            bounds,
            x + 1 + i as u16,
            row,
            ch,
            Style::default().fg(color).bg(theme::BG),
        );
    }

    popup::set_cell(
        buf,
        bounds,
        x + 1 + BAR_WIDTH as u16,
        row,
        ']',
        Style::default().fg(theme::DIM).bg(theme::BG),
    );

    popup::write_str(
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
    popup::write_str(buf, bounds, x, row, text, s);
}

fn render_mic_meter(buf: &mut Buffer, bounds: &Rect, left: u16, right: u16, row: u16, level: f32) {
    let label = "MIC ";
    let label_s = Style::default().fg(theme::DIM).bg(theme::BG);
    popup::write_str(buf, bounds, left, row, label, label_s);

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
        popup::set_cell(
            buf,
            bounds,
            bar_start + i as u16,
            row,
            ch,
            Style::default().fg(color).bg(theme::BG),
        );
    }
}
