use ratatui::Frame;
use ratatui::style::Style;

use crate::state::UserConfigState;
use crate::ui::icons::Icons;
use crate::ui::{form_field, popup, theme};

const POPUP_WIDTH: u16 = 54;
const POPUP_HEIGHT: u16 = 22;

pub fn render(
    state: &UserConfigState,
    icons: &Icons,
    frame: &mut Frame,
    phase: f32,
    cursor_visible: bool,
) {
    let area = frame.area();
    if area.width < 30 || area.height < 12 {
        return;
    }

    let popup_w = POPUP_WIDTH.min(area.width.saturating_sub(4));
    let popup_h = POPUP_HEIGHT.min(area.height.saturating_sub(4));
    let popup_area = popup::centered_rect(popup_w, popup_h, area);
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    popup::render_popup_chrome(buf, &bounds, popup_area, "PROFILE", phase);

    let left = popup_area.x + 3;
    let right = popup_area.x + popup_area.width.saturating_sub(3);
    let inner_w = (right - left) as usize;

    if state.loading {
        let msg = "Loading...";
        let mx = left + (inner_w.saturating_sub(msg.len())) as u16 / 2;
        let my = popup_area.y + popup_area.height / 2;
        popup::write_str(buf, &bounds, mx, my, msg, theme::loading_style());
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
    let (rec_text, rec_color) = match state.recovery_status {
        crate::event::RecoveryStatus::Enabled => ("yes", theme::GREEN),
        crate::event::RecoveryStatus::Incomplete => ("incomplete", theme::YELLOW),
        crate::event::RecoveryStatus::Disabled => ("no", theme::RED),
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
    form_field::render_label(
        buf,
        left,
        label_x,
        row,
        "DISPLAY NAME",
        name_selected,
        icons,
    );

    if state.editing_display_name {
        form_field::render_editing(
            buf,
            value_x,
            right,
            row,
            &state.display_name_buffer,
            cursor_visible,
        );
    } else {
        let name = state.display_name.as_deref().unwrap_or("\u{2014}");
        form_field::render_value(buf, value_x, right, row, name, name_selected);
    }
    row += 1;

    // ── Field 1: PASSWORD (action) ──
    let pw_selected = state.selected_field == 1;
    form_field::render_label(buf, left, label_x, row, "PASSWORD", pw_selected, icons);
    form_field::render_value(buf, value_x, right, row, "change...", pw_selected);
    row += 1;

    // Show saving indicator
    if state.saving {
        row += 2;
        let msg = "saving...";
        let sx = left + (inner_w.saturating_sub(msg.len())) as u16 / 2;
        popup::write_str(buf, &bounds, sx, row, msg, theme::saving_style());
    }

    // Hints
    popup::render_hint(
        buf,
        &bounds,
        popup_area,
        "j/k navigate  Enter select  Esc close",
    );
}
