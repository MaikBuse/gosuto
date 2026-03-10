use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::{App, QUICK_EMOJIS};
use crate::ui::emoji_data::{EmojiCategory, filtered_emojis};
use crate::ui::{popup, theme};

/// Each emoji slot: 1 padding + 2 emoji cols + 1 padding = 4 cols, plus 1 gap.
const SLOT_W: u16 = 5;
/// Left/right margin inside popup border.
const MARGIN: u16 = 3;
/// Number of emoji columns in the grid (matches quick row width).
const GRID_COLS: usize = 8;
/// Number of visible grid rows.
const GRID_VISIBLE_ROWS: usize = 6;

pub fn render(app: &App, frame: &mut Frame) {
    let Some(ref picker) = app.reaction_picker else {
        return;
    };

    let area = frame.area();
    let content_w = SLOT_W * QUICK_EMOJIS.len() as u16;

    // Height: border(1) + blank(1) + quick_row(1) + number_hints(1) + blank(1)
    //       + filter_row(1) + grid_rows(GRID_VISIBLE_ROWS) + blank(1)
    //       + emoji_name(1) + hint(1) + border(1) = 11 + GRID_VISIBLE_ROWS
    let popup_h = (11 + GRID_VISIBLE_ROWS as u16).min(area.height);
    let popup_w = (content_w + MARGIN * 2).min(area.width);
    let popup_rect = popup::centered_rect(popup_w, popup_h, area);
    let inner_w = popup_rect.width.saturating_sub(MARGIN * 2);

    let filtered = filtered_emojis(&picker.filter);

    // Chrome
    {
        let buf = frame.buffer_mut();
        let bounds = area;
        popup::render_popup_chrome(
            buf,
            &bounds,
            popup_rect,
            "React",
            app.room_list_anim.pulse_phase,
        );

        // Hint bar at bottom
        let hint = if picker.filter_active {
            "Enter: apply  Esc: clear"
        } else if picker.in_grid {
            "hjkl: move  /: filter  Enter: confirm"
        } else {
            "h/l: move  j: grid  /: filter  Enter: confirm"
        };
        popup::render_hint(buf, &bounds, popup_rect, hint);
    }

    // Quick row
    let quick_y = popup_rect.y + 2;
    {
        let mut emoji_spans: Vec<Span> = Vec::new();
        for (i, emoji) in QUICK_EMOJIS.iter().enumerate() {
            let is_selected =
                !picker.in_grid && !picker.filter_active && i == picker.quick_pick_index;
            let is_own = picker.existing_own_reactions.contains(&emoji.to_string());

            let style = if is_selected {
                theme::highlight_focused_style()
            } else if is_own {
                theme::reaction_own_badge_style()
            } else {
                theme::text_style()
            };

            emoji_spans.push(Span::styled(format!(" {} ", emoji), style));
            emoji_spans.push(Span::raw(" "));
        }

        let emoji_rect = Rect {
            x: popup_rect.x + MARGIN,
            y: quick_y,
            width: inner_w,
            height: 1,
        };
        frame.render_widget(Paragraph::new(Line::from(emoji_spans)), emoji_rect);

        // Number hints row
        let mut hint_spans: Vec<Span> = Vec::new();
        for i in 1..=QUICK_EMOJIS.len() {
            hint_spans.push(Span::styled(
                format!(" {:<width$}", i, width = SLOT_W as usize - 1),
                theme::dim_style(),
            ));
        }
        let hint_rect = Rect {
            x: popup_rect.x + MARGIN,
            y: quick_y + 1,
            width: inner_w,
            height: 1,
        };
        frame.render_widget(Paragraph::new(Line::from(hint_spans)), hint_rect);
    }

    // Filter row
    let filter_y = quick_y + 3;
    if picker.filter_active || !picker.filter.is_empty() {
        let buf = frame.buffer_mut();
        let bounds = area;
        let label = "Filter: ";
        popup::write_str(
            buf,
            &bounds,
            popup_rect.x + MARGIN,
            filter_y,
            label,
            theme::dim_style(),
        );
        popup::write_str(
            buf,
            &bounds,
            popup_rect.x + MARGIN + label.len() as u16,
            filter_y,
            &picker.filter,
            if picker.filter_active {
                theme::text_style()
            } else {
                theme::dim_style()
            },
        );
        if picker.filter_active {
            // Cursor indicator
            let cursor_x = popup_rect.x + MARGIN + label.len() as u16 + picker.filter.len() as u16;
            popup::write_str(buf, &bounds, cursor_x, filter_y, "_", theme::text_style());
        }
    }

    // Grid area
    let grid_y = filter_y + 1;
    let grid_rows_available = popup_rect
        .height
        .saturating_sub(grid_y - popup_rect.y)
        .saturating_sub(3) as usize; // leave room for name + hint + border

    if !filtered.is_empty() && grid_rows_available > 0 {
        // Build rows with category headers
        // Each row is either a category header or a row of emojis
        // We need to map grid_index (flat emoji index) to visual rows
        let total_emoji_rows = filtered.len().div_ceil(GRID_COLS);
        let selected_emoji_row = if picker.in_grid {
            picker.grid_index / GRID_COLS
        } else {
            0
        };

        // Build a list of visual rows: (Option<category_label>, Option<&[emoji_indices]>)
        struct VisualRow {
            category: Option<&'static str>,
            emoji_start: usize, // first emoji index in this row
            emoji_count: usize, // 0 for header-only rows
        }
        let mut visual_rows: Vec<VisualRow> = Vec::new();
        let mut i = 0;
        let mut last_cat: Option<EmojiCategory> = None;
        while i < filtered.len() {
            let cat = filtered[i].category;
            if last_cat != Some(cat) {
                visual_rows.push(VisualRow {
                    category: Some(cat.label()),
                    emoji_start: i,
                    emoji_count: 0,
                });
                last_cat = Some(cat);
            }
            let row_end = (i + GRID_COLS).min(filtered.len());
            visual_rows.push(VisualRow {
                category: None,
                emoji_start: i,
                emoji_count: row_end - i,
            });
            i = row_end;
        }

        // Find which visual row the selected emoji is on
        let selected_visual_row = if picker.in_grid {
            visual_rows
                .iter()
                .position(|vr| {
                    vr.emoji_count > 0
                        && picker.grid_index >= vr.emoji_start
                        && picker.grid_index < vr.emoji_start + vr.emoji_count
                })
                .unwrap_or(0)
        } else {
            0
        };

        // Auto-scroll so selected row is visible
        let scroll = {
            let max_scroll = visual_rows.len().saturating_sub(grid_rows_available);
            let mut s = picker.scroll_offset.min(max_scroll);
            if picker.in_grid {
                if selected_visual_row < s {
                    s = selected_visual_row;
                } else if selected_visual_row >= s + grid_rows_available {
                    s = selected_visual_row + 1 - grid_rows_available;
                }
            }
            s.min(max_scroll)
        };

        // Render visible rows
        let visible_end = (scroll + grid_rows_available).min(visual_rows.len());
        for (vi, vr) in visual_rows[scroll..visible_end].iter().enumerate() {
            let row_y = grid_y + vi as u16;
            if row_y >= popup_rect.y + popup_rect.height - 2 {
                break;
            }

            if let Some(cat_label) = vr.category {
                // Category header
                let buf = frame.buffer_mut();
                let bounds = area;
                let header = format!("── {} ", cat_label);
                let padded: String = if header.len() < inner_w as usize {
                    let remaining = inner_w as usize - header.len();
                    format!("{}{}", header, "─".repeat(remaining))
                } else {
                    header
                };
                popup::write_str(
                    buf,
                    &bounds,
                    popup_rect.x + MARGIN,
                    row_y,
                    &padded,
                    theme::dim_style(),
                );
            } else {
                // Emoji row
                let mut spans: Vec<Span> = Vec::new();
                for col in 0..GRID_COLS {
                    let emoji_idx = vr.emoji_start + col;
                    if col >= vr.emoji_count {
                        break;
                    }
                    let entry = &filtered[emoji_idx];
                    let is_selected = picker.in_grid && emoji_idx == picker.grid_index;
                    let is_own = picker
                        .existing_own_reactions
                        .contains(&entry.emoji.to_string());

                    let style = if is_selected {
                        theme::highlight_focused_style()
                    } else if is_own {
                        theme::reaction_own_badge_style()
                    } else {
                        theme::text_style()
                    };

                    spans.push(Span::styled(format!(" {} ", entry.emoji), style));
                    spans.push(Span::raw(" "));
                }

                let row_rect = Rect {
                    x: popup_rect.x + MARGIN,
                    y: row_y,
                    width: inner_w,
                    height: 1,
                };
                frame.render_widget(Paragraph::new(Line::from(spans)), row_rect);
            }
        }

        // Emoji name of selected item
        let name_y = popup_rect.y + popup_rect.height - 3;
        if picker.in_grid && picker.grid_index < filtered.len() {
            let name = filtered[picker.grid_index].name;
            let buf = frame.buffer_mut();
            let bounds = area;
            let truncated = popup::truncate_str(name, inner_w as usize);
            popup::write_str(
                buf,
                &bounds,
                popup_rect.x + MARGIN,
                name_y,
                &truncated,
                theme::dim_style(),
            );
        }

        // Update scroll_offset for next frame (mutability through interior pattern not possible,
        // but the scroll_offset is updated in the key handler)
        let _ = (scroll, total_emoji_rows, selected_emoji_row);
    }
}
