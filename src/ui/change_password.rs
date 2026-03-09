use ratatui::Frame;
use ratatui::style::{Modifier, Style};

use crate::state::ChangePasswordState;
use crate::ui::icons::Icons;
use crate::ui::popup;
use crate::ui::theme;

const POPUP_WIDTH: u16 = 44;
const POPUP_HEIGHT: u16 = 12;

const FIELDS: [&str; 3] = ["CURRENT", "NEW", "CONFIRM"];

pub fn render(state: &ChangePasswordState, icons: &Icons, frame: &mut Frame) {
    let area = frame.area();
    if area.width < 30 || area.height < 12 {
        return;
    }

    let popup_w = POPUP_WIDTH.min(area.width.saturating_sub(4));
    let popup_h = POPUP_HEIGHT.min(area.height.saturating_sub(4));
    let popup_area = popup::centered_rect(popup_w, popup_h, area);
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    popup::render_popup_chrome(buf, &bounds, popup_area, "CHANGE PASSWORD");

    let left = popup_area.x + 3;
    let right = popup_area.x + popup_area.width.saturating_sub(3);
    let inner_w = (right - left) as usize;
    let label_x = left + 2;
    let value_x = left + 17;
    let max_val_w = (right - value_x) as usize;

    let mut row = popup_area.y + 2;

    let buffers = [
        &state.current_buffer,
        &state.new_buffer,
        &state.confirm_buffer,
    ];

    for (i, field_name) in FIELDS.iter().enumerate() {
        let selected = state.selected_field == i;
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
        popup::write_str(buf, &bounds, label_x, row, field_name, label_s);

        // Masked value
        let buf_len = buffers[i].len();
        let masked: String = "\u{2022}".repeat(buf_len.min(max_val_w.saturating_sub(1)));
        let val_color = if selected { theme::GREEN } else { theme::DIM };
        popup::write_str(
            buf,
            &bounds,
            value_x,
            row,
            &masked,
            Style::default().fg(val_color).bg(theme::BG),
        );

        // Blinking cursor on selected field
        if selected {
            let cursor_x = value_x + masked.chars().count() as u16;
            let cursor_s = Style::default()
                .fg(theme::GREEN)
                .bg(theme::BG)
                .add_modifier(Modifier::SLOW_BLINK);
            popup::set_cell(buf, &bounds, cursor_x, row, '_', cursor_s);
        }

        row += 1;
    }

    // Saving indicator
    if state.saving {
        row += 1;
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

    popup::render_hint(
        buf,
        &bounds,
        popup_area,
        "j/k navigate  Enter submit  Esc close",
    );
}
