use ratatui::Frame;
use ratatui::style::{Modifier, Style};

use crate::app::UserConfigState;
use crate::ui::icons::Icons;
use crate::ui::popup;
use crate::ui::theme;

const POPUP_WIDTH: u16 = 54;
const POPUP_HEIGHT: u16 = 22;

pub fn render(state: &UserConfigState, icons: &Icons, frame: &mut Frame) {
    let area = frame.area();
    if area.width < 30 || area.height < 12 {
        return;
    }

    let popup_w = POPUP_WIDTH.min(area.width.saturating_sub(4));
    let popup_h = POPUP_HEIGHT.min(area.height.saturating_sub(4));
    let popup_area = popup::centered_rect(popup_w, popup_h, area);
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    popup::render_popup_chrome(buf, &bounds, popup_area, "PROFILE");

    let left = popup_area.x + 3;
    let right = popup_area.x + popup_area.width.saturating_sub(3);
    let inner_w = (right - left) as usize;

    if state.loading {
        let msg = "Loading...";
        let mx = left + (inner_w.saturating_sub(msg.len())) as u16 / 2;
        let my = popup_area.y + popup_area.height / 2;
        popup::write_str(
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

    let mut row = popup_area.y + 2;

    // USER ID (read-only)
    popup::write_str(buf, &bounds, label_x, row, "USER ID", label_s);
    let id_display = popup::truncate_str(&state.user_id, (right - value_x) as usize);
    popup::write_str(buf, &bounds, value_x, row, &id_display, value_s);
    row += 1;

    // DEVICE ID (read-only)
    popup::write_str(buf, &bounds, label_x, row, "DEVICE ID", label_s);
    let dev_display = popup::truncate_str(&state.device_id, (right - value_x) as usize);
    popup::write_str(buf, &bounds, value_x, row, &dev_display, value_s);
    row += 1;

    // HOMESERVER (read-only)
    popup::write_str(buf, &bounds, label_x, row, "HOMESERVER", label_s);
    let hs_display = popup::truncate_str(&state.homeserver, (right - value_x) as usize);
    popup::write_str(buf, &bounds, value_x, row, &hs_display, value_s);
    row += 1;

    // VERIFIED (read-only)
    popup::write_str(buf, &bounds, label_x, row, "VERIFIED", label_s);
    let (ver_text, ver_color) = if state.verified {
        ("yes", theme::GREEN)
    } else {
        ("no", theme::RED)
    };
    popup::write_str(
        buf,
        &bounds,
        value_x,
        row,
        ver_text,
        Style::default().fg(ver_color).bg(theme::BG),
    );
    row += 1;

    // RECOVERY (read-only)
    popup::write_str(buf, &bounds, label_x, row, "RECOVERY", label_s);
    let (rec_text, rec_color) = if state.recovery_enabled {
        ("yes", theme::GREEN)
    } else {
        ("no", theme::RED)
    };
    popup::write_str(
        buf,
        &bounds,
        value_x,
        row,
        rec_text,
        Style::default().fg(rec_color).bg(theme::BG),
    );
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

    popup::write_str(buf, &bounds, left, row, name_marker, name_marker_s);
    popup::set_cell(
        buf,
        &bounds,
        left + 1,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );
    popup::write_str(buf, &bounds, label_x, row, "DISPLAY NAME", name_label_s);

    let max_name_w = (right - value_x) as usize;
    if state.editing_display_name {
        let display = popup::truncate_str(&state.display_name_buffer, max_name_w.saturating_sub(1));
        let edit_s = Style::default().fg(theme::GREEN).bg(theme::BG);
        popup::write_str(buf, &bounds, value_x, row, &display, edit_s);
        // Cursor underscore
        let cursor_x = value_x + display.chars().count() as u16;
        let cursor_s = Style::default()
            .fg(theme::GREEN)
            .bg(theme::BG)
            .add_modifier(Modifier::SLOW_BLINK);
        popup::set_cell(buf, &bounds, cursor_x, row, '_', cursor_s);
    } else {
        let name = state.display_name.as_deref().unwrap_or("\u{2014}");
        let name_display = popup::truncate_str(name, max_name_w);
        let name_val_color = if name_selected {
            theme::TEXT
        } else {
            theme::DIM
        };
        let name_val_s = Style::default().fg(name_val_color).bg(theme::BG);
        popup::write_str(buf, &bounds, value_x, row, &name_display, name_val_s);
    }
    row += 1;

    // ── Field 1: PASSWORD (action) ──
    let pw_selected = state.selected_field == 1;
    let pw_marker_color = if pw_selected { theme::CYAN } else { theme::DIM };
    let pw_label_color = if pw_selected {
        theme::CYAN
    } else {
        theme::TEXT
    };
    let pw_marker = if pw_selected {
        icons.selected
    } else {
        icons.unselected
    };

    let pw_marker_s = Style::default().fg(pw_marker_color).bg(theme::BG);
    let pw_label_s = Style::default()
        .fg(pw_label_color)
        .bg(theme::BG)
        .add_modifier(if pw_selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });

    popup::write_str(buf, &bounds, left, row, pw_marker, pw_marker_s);
    popup::set_cell(
        buf,
        &bounds,
        left + 1,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );
    popup::write_str(buf, &bounds, label_x, row, "PASSWORD", pw_label_s);

    let pw_hint_color = if pw_selected { theme::TEXT } else { theme::DIM };
    popup::write_str(
        buf,
        &bounds,
        value_x,
        row,
        "change...",
        Style::default().fg(pw_hint_color).bg(theme::BG),
    );
    row += 1;

    // Show saving indicator
    if state.saving {
        row += 2;
        let msg = "saving...";
        let sx = left + (inner_w.saturating_sub(msg.len())) as u16 / 2;
        popup::write_str(
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
    popup::render_hint(
        buf,
        &bounds,
        popup_area,
        "j/k navigate  Enter select  Esc close",
    );
}
