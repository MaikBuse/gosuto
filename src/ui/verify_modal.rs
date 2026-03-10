use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::state::{VerificationModalState, VerificationStage};
use crate::ui::theme;
use crate::ui::{form_field, popup};

const POPUP_WIDTH: u16 = 48;
const POPUP_HEIGHT: u16 = 14;
const MENU_HEIGHT: u16 = 12;

pub fn render(state: &VerificationModalState, frame: &mut Frame, phase: f32) {
    let area = frame.area();
    if area.width < 30 || area.height < 10 {
        return;
    }

    let popup_w = POPUP_WIDTH.min(area.width.saturating_sub(4));
    let popup_h = match state.stage {
        VerificationStage::ChooseAction { .. } | VerificationStage::EnterUserId => {
            MENU_HEIGHT.min(area.height.saturating_sub(4))
        }
        _ => POPUP_HEIGHT.min(area.height.saturating_sub(4)),
    };
    let popup_area = popup::centered_rect(popup_w, popup_h, area);
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    popup::render_popup_chrome(buf, &bounds, popup_area, "VERIFICATION", phase);

    let left = popup_area.x + 3;
    let right = popup_area.x + popup_area.width.saturating_sub(3);
    let inner_w = (right - left) as usize;

    match &state.stage {
        VerificationStage::ChooseAction { selected } => {
            let options = [
                "Verify this device",
                "Verify a user",
                "Reset cross-signing keys",
            ];
            let y_start = popup_area.y + 3;

            for (i, label) in options.iter().enumerate() {
                let sel = *selected == i as u8;
                let marker = if sel { "▸ " } else { "  " };
                let text = format!("{marker}{label}");
                let color = if sel { theme::CYAN } else { theme::TEXT };
                let style = if sel {
                    Style::default()
                        .fg(color)
                        .bg(theme::BG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color).bg(theme::BG)
                };
                popup::write_str(buf, &bounds, left, y_start + i as u16 * 2, &text, style);
            }

            popup::render_hint(
                buf,
                &bounds,
                popup_area,
                "j/k navigate  Enter select  Esc cancel",
            );
        }
        VerificationStage::EnterUserId => {
            let label = "User ID:";
            let label_y = popup_area.y + 3;
            popup::write_str(
                buf,
                &bounds,
                left,
                label_y,
                label,
                Style::default().fg(theme::TEXT).bg(theme::BG),
            );

            let value_x = left;
            let value_y = label_y + 2;
            form_field::render_editing(buf, value_x, right, value_y, &state.user_id_buffer);

            popup::render_hint(buf, &bounds, popup_area, "Enter verify  Esc back");
        }
        VerificationStage::WaitingForOtherDevice => {
            let line1 = format!(
                "Verifying: {}",
                popup::truncate_str(&state.sender, inner_w - 12)
            );
            let line2 = "Waiting for other device...";
            let y = popup_area.y + popup_area.height / 2 - 1;
            let x1 = left + (inner_w.saturating_sub(line1.len())) as u16 / 2;
            let x2 = left + (inner_w.saturating_sub(line2.len())) as u16 / 2;
            popup::write_str(
                buf,
                &bounds,
                x1,
                y,
                &line1,
                Style::default().fg(theme::TEXT).bg(theme::BG),
            );
            popup::write_str(
                buf,
                &bounds,
                x2,
                y + 2,
                line2,
                Style::default()
                    .fg(theme::CYAN)
                    .bg(theme::BG)
                    .add_modifier(Modifier::SLOW_BLINK),
            );

            popup::render_hint(buf, &bounds, popup_area, "Esc cancel");
        }
        VerificationStage::EmojiConfirmation => {
            // Title line
            let title_msg = "Compare emoji on both devices:";
            let tx = left + (inner_w.saturating_sub(title_msg.len())) as u16 / 2;
            popup::write_str(
                buf,
                &bounds,
                tx,
                popup_area.y + 2,
                title_msg,
                Style::default().fg(theme::TEXT).bg(theme::BG),
            );

            // Render emoji in two rows: symbols on one line, descriptions below
            let emoji_count = state.emojis.len();
            if emoji_count > 0 {
                // Row 1: symbols (4 per row)
                let row1_count = 4.min(emoji_count);
                let row2_count = emoji_count.saturating_sub(4);

                let emoji_y = popup_area.y + 4;

                // First row of symbols
                render_emoji_row(
                    buf,
                    &bounds,
                    &state.emojis[..row1_count],
                    left,
                    inner_w,
                    emoji_y,
                );

                // Second row of symbols (if any)
                if row2_count > 0 {
                    render_emoji_row(
                        buf,
                        &bounds,
                        &state.emojis[row1_count..],
                        left,
                        inner_w,
                        emoji_y + 3,
                    );
                }
            }

            // Prompt
            let prompt = "Do these match? [y/n]";
            let px = left + (inner_w.saturating_sub(prompt.len())) as u16 / 2;
            popup::write_str(
                buf,
                &bounds,
                px,
                popup_area.y + popup_area.height - 3,
                prompt,
                Style::default()
                    .fg(theme::GREEN)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            );

            popup::render_hint(buf, &bounds, popup_area, "y confirm  n reject  Esc cancel");
        }
        VerificationStage::Completed => {
            let msg = "Verification successful!";
            let x = left + (inner_w.saturating_sub(msg.len())) as u16 / 2;
            let y = popup_area.y + popup_area.height / 2;
            popup::write_str(
                buf,
                &bounds,
                x,
                y,
                msg,
                Style::default()
                    .fg(theme::GREEN)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            );

            popup::render_hint(buf, &bounds, popup_area, "Enter/Esc close");
        }
        VerificationStage::Failed(reason) => {
            let msg = "Verification failed";
            let x = left + (inner_w.saturating_sub(msg.len())) as u16 / 2;
            let y = popup_area.y + popup_area.height / 2 - 1;
            popup::write_str(
                buf,
                &bounds,
                x,
                y,
                msg,
                Style::default()
                    .fg(theme::RED)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            );

            let reason_display = popup::truncate_str(reason, inner_w);
            let rx = left + (inner_w.saturating_sub(reason_display.len())) as u16 / 2;
            popup::write_str(
                buf,
                &bounds,
                rx,
                y + 2,
                &reason_display,
                Style::default().fg(theme::DIM).bg(theme::BG),
            );

            popup::render_hint(buf, &bounds, popup_area, "Enter/Esc close");
        }
    }
}

fn render_emoji_row(
    buf: &mut Buffer,
    bounds: &Rect,
    emojis: &[(String, String)],
    left: u16,
    inner_w: usize,
    y: u16,
) {
    let count = emojis.len();
    if count == 0 {
        return;
    }

    // Calculate spacing: each emoji slot is ~10 chars wide
    let slot_width = (inner_w / count).min(10);
    let total_width = slot_width * count;
    let start_x = left + (inner_w.saturating_sub(total_width)) as u16 / 2;

    for (i, (_symbol, description)) in emojis.iter().enumerate() {
        let slot_x = start_x + (i * slot_width) as u16;

        let desc = popup::truncate_str(description, slot_width);
        let desc_offset = (slot_width.saturating_sub(desc.len())) / 2;
        popup::write_str(
            buf,
            bounds,
            slot_x + desc_offset as u16,
            y,
            &desc,
            Style::default().fg(theme::TEXT).bg(theme::BG),
        );
    }
}
