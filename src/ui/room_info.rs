use ratatui::Frame;
use ratatui::style::{Color, Modifier, Style};

use crate::state::{HISTORY_VISIBILITY_OPTIONS, RoomInfoState};
use crate::ui::icons::Icons;
use crate::ui::popup;
use crate::ui::theme;

const POPUP_WIDTH: u16 = 54;
const POPUP_HEIGHT: u16 = 20;

pub fn render(state: &RoomInfoState, icons: &Icons, frame: &mut Frame) {
    let area = frame.area();
    if area.width < 30 || area.height < 12 {
        return;
    }

    let popup_w = POPUP_WIDTH.min(area.width.saturating_sub(4));
    let popup_h = POPUP_HEIGHT.min(area.height.saturating_sub(4));
    let popup_area = popup::centered_rect(popup_w, popup_h, area);
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    popup::render_popup_chrome(buf, &bounds, popup_area, "ROOM EDIT");

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
    let value_x = left + 15;

    let mut row = popup_area.y + 2;

    // Room ID (read-only)
    popup::write_str(buf, &bounds, label_x, row, "ROOM ID", label_s);
    let id_display = popup::truncate_str(&state.room_id, (right - value_x) as usize);
    popup::write_str(buf, &bounds, value_x, row, &id_display, value_s);
    row += 1;

    row += 1;

    // ── Field 0: NAME (editable) ──
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
    popup::write_str(buf, &bounds, label_x, row, "NAME", name_label_s);

    let max_name_w = (right - value_x) as usize;
    if state.editing_name {
        // Render editable name buffer with cursor
        let display = popup::truncate_str(&state.name_buffer, max_name_w.saturating_sub(1));
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
        let name = state.name.as_deref().unwrap_or("\u{2014}");
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

    // ── Field 1: TOPIC (editable) ──
    let topic_selected = state.selected_field == 1;
    let topic_marker_color = if topic_selected {
        theme::CYAN
    } else {
        theme::DIM
    };
    let topic_label_color = if topic_selected {
        theme::CYAN
    } else {
        theme::TEXT
    };
    let topic_marker = if topic_selected {
        icons.selected
    } else {
        icons.unselected
    };

    let topic_marker_s = Style::default().fg(topic_marker_color).bg(theme::BG);
    let topic_label_s = Style::default()
        .fg(topic_label_color)
        .bg(theme::BG)
        .add_modifier(if topic_selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });

    popup::write_str(buf, &bounds, left, row, topic_marker, topic_marker_s);
    popup::set_cell(
        buf,
        &bounds,
        left + 1,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );
    popup::write_str(buf, &bounds, label_x, row, "TOPIC", topic_label_s);

    let max_topic_w = (right - value_x) as usize;
    if state.editing_topic {
        let display = popup::truncate_str(&state.topic_buffer, max_topic_w.saturating_sub(1));
        let edit_s = Style::default().fg(theme::GREEN).bg(theme::BG);
        popup::write_str(buf, &bounds, value_x, row, &display, edit_s);
        let cursor_x = value_x + display.chars().count() as u16;
        let cursor_s = Style::default()
            .fg(theme::GREEN)
            .bg(theme::BG)
            .add_modifier(Modifier::SLOW_BLINK);
        popup::set_cell(buf, &bounds, cursor_x, row, '_', cursor_s);
    } else {
        let topic = state.topic.as_deref().unwrap_or("\u{2014}");
        let topic_display = popup::truncate_str(topic, max_topic_w);
        let topic_val_color = if topic_selected {
            theme::TEXT
        } else {
            theme::DIM
        };
        let topic_val_s = Style::default().fg(topic_val_color).bg(theme::BG);
        popup::write_str(buf, &bounds, value_x, row, &topic_display, topic_val_s);
    }
    row += 1;

    // ── Field 2: HISTORY (editable, cycle selector) ──
    let hist_selected = state.selected_field == 2;
    let hist_marker_color = if hist_selected {
        theme::CYAN
    } else {
        theme::DIM
    };
    let hist_label_color = if hist_selected {
        theme::CYAN
    } else {
        theme::TEXT
    };
    let hist_marker = if hist_selected {
        icons.selected
    } else {
        icons.unselected
    };

    let hist_marker_s = Style::default().fg(hist_marker_color).bg(theme::BG);
    let hist_label_s = Style::default()
        .fg(hist_label_color)
        .bg(theme::BG)
        .add_modifier(if hist_selected {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });

    popup::write_str(buf, &bounds, left, row, hist_marker, hist_marker_s);
    popup::set_cell(
        buf,
        &bounds,
        left + 1,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );
    popup::write_str(buf, &bounds, label_x, row, "HISTORY", hist_label_s);

    // Render selector with arrows
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

    popup::write_str(buf, &bounds, value_x, row, icons.arrow_left, arrow_s);
    popup::set_cell(
        buf,
        &bounds,
        value_x + 1,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );

    let vis_display = &state.history_visibility;
    popup::write_str(buf, &bounds, value_x + 2, row, vis_display, vis_s);

    let end_x = value_x + 2 + vis_display.chars().count() as u16;
    popup::set_cell(
        buf,
        &bounds,
        end_x,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );
    popup::write_str(buf, &bounds, end_x + 1, row, icons.arrow_right, arrow_s);
    row += 1;

    // History visibility description
    let desc = popup::history_visibility_description(&state.history_visibility);
    let desc_s = Style::default().fg(theme::DIM).bg(theme::BG);
    popup::write_str(buf, &bounds, value_x, row, desc, desc_s);
    row += 1;

    // ── Field 3: ENCRYPTED (editable when unencrypted, read-only when encrypted) ──
    if state.encrypted {
        // Read-only: show "yes" in green, no marker, not selectable
        popup::write_str(buf, &bounds, label_x, row, "ENCRYPTED", label_s);
        popup::write_str(
            buf,
            &bounds,
            value_x,
            row,
            "yes",
            Style::default().fg(theme::GREEN).bg(theme::BG),
        );
    } else {
        let enc_selected = state.selected_field == 3;
        let enc_marker_color = if enc_selected {
            theme::CYAN
        } else {
            theme::DIM
        };
        let enc_label_color = if enc_selected {
            theme::CYAN
        } else {
            theme::TEXT
        };
        let enc_marker = if enc_selected {
            icons.selected
        } else {
            icons.unselected
        };

        let enc_marker_s = Style::default().fg(enc_marker_color).bg(theme::BG);
        let enc_label_s = Style::default()
            .fg(enc_label_color)
            .bg(theme::BG)
            .add_modifier(if enc_selected {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });

        popup::write_str(buf, &bounds, left, row, enc_marker, enc_marker_s);
        popup::set_cell(
            buf,
            &bounds,
            left + 1,
            row,
            ' ',
            Style::default().bg(theme::BG),
        );
        popup::write_str(buf, &bounds, label_x, row, "ENCRYPTED", enc_label_s);

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

        popup::write_str(buf, &bounds, value_x, row, icons.arrow_left, enc_arrow_s);
        popup::set_cell(
            buf,
            &bounds,
            value_x + 1,
            row,
            ' ',
            Style::default().bg(theme::BG),
        );

        let enc_display = &state.encryption_selection;
        popup::write_str(buf, &bounds, value_x + 2, row, enc_display, enc_val_s);

        let enc_end_x = value_x + 2 + enc_display.chars().count() as u16;
        popup::set_cell(
            buf,
            &bounds,
            enc_end_x,
            row,
            ' ',
            Style::default().bg(theme::BG),
        );
        popup::write_str(
            buf,
            &bounds,
            enc_end_x + 1,
            row,
            icons.arrow_right,
            enc_arrow_s,
        );
    }

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

    // Show valid options hint
    row = popup_area.y + popup_area.height.saturating_sub(4);
    let opts: String = if state.selected_field == 3 && !state.encrypted {
        "no | yes".to_string()
    } else {
        HISTORY_VISIBILITY_OPTIONS.join(" | ")
    };
    let opts_x = left + (inner_w.saturating_sub(opts.len())) as u16 / 2;
    popup::write_str(
        buf,
        &bounds,
        opts_x,
        row,
        &opts,
        Style::default().fg(Color::Rgb(60, 60, 80)).bg(theme::BG),
    );

    // Hints
    popup::render_hint(
        buf,
        &bounds,
        popup_area,
        "j/k navigate  Enter edit  Esc close",
    );
}
