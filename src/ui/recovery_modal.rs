use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::app::{HealingStep, RecoveryModalState, RecoveryStage};
use crate::ui::theme;

const POPUP_WIDTH: u16 = 56;
const POPUP_HEIGHT: u16 = 14;

pub fn render(state: &RecoveryModalState, frame: &mut Frame) {
    let area = frame.area();
    if area.width < 30 || area.height < 10 {
        return;
    }

    let popup_w = POPUP_WIDTH.min(area.width.saturating_sub(4));
    let popup_h = POPUP_HEIGHT.min(area.height.saturating_sub(4));
    let popup = centered_rect(popup_w, popup_h, area);
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    fill_bg(buf, &bounds, popup);
    render_border(buf, &bounds, popup, theme::CYAN);
    render_title(buf, &bounds, popup, theme::CYAN);

    let left = popup.x + 3;
    let inner_w = (popup.width.saturating_sub(6)) as usize;
    let mut y = popup.y + 3;

    let text_style = Style::default().fg(theme::TEXT).bg(theme::BG);
    let dim_style = Style::default().fg(theme::DIM).bg(theme::BG);
    let key_style = Style::default()
        .fg(theme::MAGENTA)
        .bg(theme::BG)
        .add_modifier(Modifier::BOLD);
    let green_style = Style::default().fg(theme::GREEN).bg(theme::BG);
    let red_style = Style::default().fg(theme::RED).bg(theme::BG);

    match &state.stage {
        RecoveryStage::Checking => {
            write_str(
                buf,
                &bounds,
                left,
                y,
                "Checking recovery status...",
                dim_style,
            );
        }
        RecoveryStage::Disabled => {
            write_str(buf, &bounds, left, y, "No recovery key found", text_style);
            y += 2;
            write_str(buf, &bounds, left, y, "[Enter]", key_style);
            write_str(buf, &bounds, left + 8, y, "Create recovery key", text_style);
        }
        RecoveryStage::Enabled => {
            write_str(buf, &bounds, left, y, "Recovery is enabled", green_style);
            y += 2;
            write_str(buf, &bounds, left, y, "[r]", key_style);
            write_str(buf, &bounds, left + 4, y, "Reset key", text_style);
        }
        RecoveryStage::Incomplete => {
            write_str(
                buf,
                &bounds,
                left,
                y,
                "Recovery exists but not cached locally",
                text_style,
            );
            y += 2;
            write_str(buf, &bounds, left, y, "[e]", key_style);
            write_str(buf, &bounds, left + 4, y, "Enter key", text_style);
            write_str(buf, &bounds, left + 16, y, "[r]", key_style);
            write_str(buf, &bounds, left + 20, y, "Reset", text_style);
        }
        RecoveryStage::EnterKey => {
            write_str(buf, &bounds, left, y, "Enter recovery key:", text_style);
            y += 2;
            let display = if state.key_buffer.is_empty() {
                "\u{2588}".to_string()
            } else {
                let visible: String = state
                    .key_buffer
                    .chars()
                    .take(inner_w.saturating_sub(1))
                    .collect();
                format!("{visible}\u{2588}")
            };
            write_str(buf, &bounds, left, y, &display, green_style);
            y += 2;
            write_str(buf, &bounds, left, y, "[Enter]", key_style);
            write_str(buf, &bounds, left + 8, y, "Submit", dim_style);
        }
        RecoveryStage::NeedPassword => {
            write_str(buf, &bounds, left, y, "Password required for", text_style);
            y += 1;
            write_str(buf, &bounds, left, y, "cross-signing reset:", text_style);
            y += 2;
            let masked: String =
                "\u{2022}".repeat(state.password_buffer.len().min(inner_w.saturating_sub(1)));
            let display = format!("{masked}\u{2588}");
            write_str(buf, &bounds, left, y, &display, green_style);
            y += 2;
            write_str(buf, &bounds, left, y, "[Enter]", key_style);
            write_str(buf, &bounds, left + 8, y, "Submit", dim_style);
        }
        RecoveryStage::Healing(step) => {
            let msg = match step {
                HealingStep::CrossSigning => "Setting up cross-signing...",
                HealingStep::Backup => "Enabling backup...",
                HealingStep::ExportSecrets => "Exporting secrets to new recovery key...",
            };
            write_str(buf, &bounds, left, y, msg, dim_style);
        }
        RecoveryStage::Recovering => {
            write_str(
                buf,
                &bounds,
                left,
                y,
                "Recovering from phrase...",
                dim_style,
            );
        }
        RecoveryStage::Creating => {
            write_str(buf, &bounds, left, y, "Creating recovery key...", dim_style);
        }
        RecoveryStage::ShowKey(key) => {
            write_str(buf, &bounds, left, y, "Save this recovery key:", text_style);
            y += 2;
            let truncated: String = key.chars().take(inner_w).collect();
            write_str(buf, &bounds, left, y, &truncated, green_style);
            y += 2;
            write_str(buf, &bounds, left, y, "[c]", key_style);
            let copy_label = if state.copied { "Copied!" } else { "Copy" };
            write_str(buf, &bounds, left + 4, y, copy_label, text_style);
            write_str(buf, &bounds, left + 16, y, "[Enter]", key_style);
            write_str(buf, &bounds, left + 24, y, "Done", text_style);
        }
        RecoveryStage::ConfirmReset => {
            write_str(
                buf,
                &bounds,
                left,
                y,
                "Type 'yes' to confirm reset:",
                red_style,
            );
            y += 2;
            let display = if state.confirm_buffer.is_empty() {
                "\u{2588}".to_string()
            } else {
                let visible: String = state
                    .confirm_buffer
                    .chars()
                    .take(inner_w.saturating_sub(1))
                    .collect();
                format!("{visible}\u{2588}")
            };
            write_str(buf, &bounds, left, y, &display, text_style);
        }
        RecoveryStage::Resetting => {
            write_str(
                buf,
                &bounds,
                left,
                y,
                "Resetting recovery key...",
                dim_style,
            );
        }
        RecoveryStage::Error(msg) => {
            write_str(buf, &bounds, left, y, msg, red_style);
            y += 2;
            write_str(buf, &bounds, left, y, "[Enter]", key_style);
            write_str(buf, &bounds, left + 8, y, "Close", dim_style);
        }
    }

    // Hint at bottom
    let hint = "Esc close";
    let hx = popup.x + (popup_w.saturating_sub(hint.len() as u16)) / 2;
    write_str(buf, &bounds, hx, popup.y + popup_h - 2, hint, dim_style);
}

// ── Helpers ───────────────────────────────────────────

fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

fn fill_bg(buf: &mut Buffer, bounds: &Rect, popup: Rect) {
    for y in popup.y..popup.y + popup.height {
        for x in popup.x..popup.x + popup.width {
            if in_bounds(x, y, bounds) {
                let cell = &mut buf[(x, y)];
                cell.set_char(' ');
                cell.set_style(Style::default().bg(theme::BG));
                cell.skip = false;
            }
        }
    }
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

fn render_border(buf: &mut Buffer, bounds: &Rect, area: Rect, color: ratatui::style::Color) {
    let s = Style::default().fg(color).bg(theme::BG);
    let x1 = area.x;
    let x2 = area.x + area.width - 1;
    let y1 = area.y;
    let y2 = area.y + area.height - 1;

    set_cell(buf, bounds, x1, y1, '\u{2554}', s);
    set_cell(buf, bounds, x2, y1, '\u{2557}', s);
    set_cell(buf, bounds, x1, y2, '\u{255a}', s);
    set_cell(buf, bounds, x2, y2, '\u{255d}', s);

    for x in (x1 + 1)..x2 {
        set_cell(buf, bounds, x, y1, '\u{2550}', s);
        set_cell(buf, bounds, x, y2, '\u{2550}', s);
    }
    for y in (y1 + 1)..y2 {
        set_cell(buf, bounds, x1, y, '\u{2551}', s);
        set_cell(buf, bounds, x2, y, '\u{2551}', s);
    }

    let gx = x2.saturating_sub(5);
    if gx > x1 {
        set_cell(buf, bounds, gx, y2, '\u{25c8}', s);
    }
}

fn render_title(buf: &mut Buffer, bounds: &Rect, area: Rect, color: ratatui::style::Color) {
    let border_s = Style::default().fg(color).bg(theme::BG);
    let title_s = border_s.add_modifier(Modifier::BOLD);
    let title = "RECOVERY";

    let bracket_l = area.x + 3;
    let title_start = bracket_l + 2;

    set_cell(buf, bounds, bracket_l, area.y, '\u{2561}', border_s);
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
        set_cell(buf, bounds, bracket_r, area.y, '\u{255e}', border_s);
    }
}
