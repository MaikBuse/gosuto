use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::state::{VerificationModalState, VerificationStage};
use crate::ui::theme;

const POPUP_WIDTH: u16 = 48;
const POPUP_HEIGHT: u16 = 14;

pub fn render(state: &VerificationModalState, frame: &mut Frame) {
    let area = frame.area();
    if area.width < 30 || area.height < 10 {
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
                let cell = &mut buf[(x, y)];
                cell.set_char(' ');
                cell.set_style(Style::default().bg(theme::BG));
                cell.skip = false;
            }
        }
    }

    let border_color = theme::CYAN;
    render_border(buf, &bounds, popup, border_color);
    render_title(buf, &bounds, popup, border_color);

    let left = popup.x + 3;
    let right = popup.x + popup.width.saturating_sub(3);
    let inner_w = (right - left) as usize;

    match &state.stage {
        VerificationStage::WaitingForOtherDevice => {
            let line1 = format!("Verifying: {}", truncate_str(&state.sender, inner_w - 12));
            let line2 = "Waiting for other device...";
            let y = popup.y + popup.height / 2 - 1;
            let x1 = left + (inner_w.saturating_sub(line1.len())) as u16 / 2;
            let x2 = left + (inner_w.saturating_sub(line2.len())) as u16 / 2;
            write_str(
                buf,
                &bounds,
                x1,
                y,
                &line1,
                Style::default().fg(theme::TEXT).bg(theme::BG),
            );
            write_str(
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

            // Hint
            let hint = "Esc cancel";
            let hx = left + (inner_w.saturating_sub(hint.len())) as u16 / 2;
            write_str(
                buf,
                &bounds,
                hx,
                popup.y + popup.height - 2,
                hint,
                Style::default().fg(theme::DIM).bg(theme::BG),
            );
        }
        VerificationStage::EmojiConfirmation => {
            // Title line
            let title_msg = "Compare emoji on both devices:";
            let tx = left + (inner_w.saturating_sub(title_msg.len())) as u16 / 2;
            write_str(
                buf,
                &bounds,
                tx,
                popup.y + 2,
                title_msg,
                Style::default().fg(theme::TEXT).bg(theme::BG),
            );

            // Render emoji in two rows: symbols on one line, descriptions below
            let emoji_count = state.emojis.len();
            if emoji_count > 0 {
                // Row 1: symbols (4 per row)
                let row1_count = 4.min(emoji_count);
                let row2_count = emoji_count.saturating_sub(4);

                let emoji_y = popup.y + 4;

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
            write_str(
                buf,
                &bounds,
                px,
                popup.y + popup.height - 3,
                prompt,
                Style::default()
                    .fg(theme::GREEN)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            );

            // Hint
            let hint = "y confirm  n reject  Esc cancel";
            let hx = left + (inner_w.saturating_sub(hint.len())) as u16 / 2;
            write_str(
                buf,
                &bounds,
                hx,
                popup.y + popup.height - 2,
                hint,
                Style::default().fg(theme::DIM).bg(theme::BG),
            );
        }
        VerificationStage::Completed => {
            let msg = "Verification successful!";
            let x = left + (inner_w.saturating_sub(msg.len())) as u16 / 2;
            let y = popup.y + popup.height / 2;
            write_str(
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

            let hint = "Enter/Esc close";
            let hx = left + (inner_w.saturating_sub(hint.len())) as u16 / 2;
            write_str(
                buf,
                &bounds,
                hx,
                popup.y + popup.height - 2,
                hint,
                Style::default().fg(theme::DIM).bg(theme::BG),
            );
        }
        VerificationStage::Failed(reason) => {
            let msg = "Verification failed";
            let x = left + (inner_w.saturating_sub(msg.len())) as u16 / 2;
            let y = popup.y + popup.height / 2 - 1;
            write_str(
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

            let reason_display = truncate_str(reason, inner_w);
            let rx = left + (inner_w.saturating_sub(reason_display.len())) as u16 / 2;
            write_str(
                buf,
                &bounds,
                rx,
                y + 2,
                &reason_display,
                Style::default().fg(theme::DIM).bg(theme::BG),
            );

            let hint = "Enter/Esc close";
            let hx = left + (inner_w.saturating_sub(hint.len())) as u16 / 2;
            write_str(
                buf,
                &bounds,
                hx,
                popup.y + popup.height - 2,
                hint,
                Style::default().fg(theme::DIM).bg(theme::BG),
            );
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

        let desc = truncate_str(description, slot_width);
        let desc_offset = (slot_width.saturating_sub(desc.len())) / 2;
        write_str(
            buf,
            bounds,
            slot_x + desc_offset as u16,
            y,
            &desc,
            Style::default().fg(theme::TEXT).bg(theme::BG),
        );
    }
}

// ── helpers ──────────────────────────────────────────

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
        cell.skip = false;
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

    // Decorative glyph
    let gx = x2.saturating_sub(5);
    if gx > x1 {
        set_cell(buf, bounds, gx, y2, '◈', s);
    }
}

fn render_title(buf: &mut Buffer, bounds: &Rect, area: Rect, color: Color) {
    let title = "VERIFICATION";
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
