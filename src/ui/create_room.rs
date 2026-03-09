use ratatui::Frame;
use ratatui::style::{Modifier, Style};

use crate::state::{CreateRoomState, HISTORY_VISIBILITY_OPTIONS};
use crate::ui::icons::Icons;
use crate::ui::{form_field, popup, theme};

const POPUP_WIDTH: u16 = 54;
const POPUP_HEIGHT: u16 = 22;

pub fn render(state: &CreateRoomState, icons: &Icons, frame: &mut Frame) {
    let area = frame.area();
    if area.width < 30 || area.height < 12 {
        return;
    }

    let popup_w = POPUP_WIDTH.min(area.width.saturating_sub(4));
    let popup_h = POPUP_HEIGHT.min(area.height.saturating_sub(4));
    let popup_area = popup::centered_rect(popup_w, popup_h, area);
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    popup::render_popup_chrome(buf, &bounds, popup_area, "ROOM CREATE");

    let left = popup_area.x + 3;
    let right = popup_area.x + popup_area.width.saturating_sub(3);
    let inner_w = (right - left) as usize;

    let label_x = left + 2;
    let value_x = left + 15;

    let mut row = popup_area.y + 3;

    // ── Field 0: NAME (editable) ──
    let name_selected = state.selected_field == 0;
    form_field::render_label(buf, left, label_x, row, "NAME", name_selected, icons);

    if state.editing_name {
        form_field::render_editing(buf, value_x, right, row, &state.name_buffer);
    } else {
        let name = if state.name_buffer.is_empty() {
            "\u{2014}"
        } else {
            &state.name_buffer
        };
        form_field::render_value(buf, value_x, right, row, name, name_selected);
    }
    row += 1;

    // ── Field 1: TOPIC (editable) ──
    let topic_selected = state.selected_field == 1;
    form_field::render_label(buf, left, label_x, row, "TOPIC", topic_selected, icons);

    if state.editing_topic {
        form_field::render_editing(buf, value_x, right, row, &state.topic_buffer);
    } else {
        let topic = if state.topic_buffer.is_empty() {
            "\u{2014}"
        } else {
            &state.topic_buffer
        };
        form_field::render_value(buf, value_x, right, row, topic, topic_selected);
    }
    row += 1;

    // ── Field 2: HISTORY (cycle selector) ──
    let hist_selected = state.selected_field == 2;
    form_field::render_label(buf, left, label_x, row, "HISTORY", hist_selected, icons);
    form_field::render_cycle_selector(
        buf,
        value_x,
        row,
        &state.history_visibility,
        hist_selected,
        icons,
    );
    row += 1;

    // History visibility description
    let desc = popup::history_visibility_description(&state.history_visibility);
    let desc_s = Style::default().fg(theme::DIM).bg(theme::BG);
    popup::write_str(buf, &bounds, value_x, row, desc, desc_s);
    row += 1;

    // ── Field 3: ENCRYPTED (toggle) ──
    let enc_selected = state.selected_field == 3;
    form_field::render_label(buf, left, label_x, row, "ENCRYPTED", enc_selected, icons);
    form_field::render_cycle_selector(buf, value_x, row, &state.encrypted, enc_selected, icons);
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
    popup::write_str(buf, &bounds, btn_x, row, btn_label, btn_style);

    // Show valid options hint
    let opts_row = popup_area.y + popup_area.height.saturating_sub(4);
    let opts: String = if state.selected_field == 3 {
        "no | yes".to_string()
    } else {
        HISTORY_VISIBILITY_OPTIONS.join(" | ")
    };
    let opts_x = left + (inner_w.saturating_sub(opts.len())) as u16 / 2;
    popup::write_str(
        buf,
        &bounds,
        opts_x,
        opts_row,
        &opts,
        Style::default().fg(theme::MUTED).bg(theme::BG),
    );

    // Hints
    popup::render_hint(
        buf,
        &bounds,
        popup_area,
        "j/k navigate  Enter edit  Esc close",
    );
}
