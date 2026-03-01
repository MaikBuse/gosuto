use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::state::{RecoveryModalState, RecoveryStage};
use crate::ui::theme;

const POPUP_WIDTH: u16 = 52;
const POPUP_HEIGHT: u16 = 12;

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

    // Fill background
    for y in popup.y..popup.y + popup.height {
        for x in popup.x..popup.x + popup.width {
            if in_bounds(x, y, &bounds) {
                buf[(x, y)].set_char(' ');
                buf[(x, y)].set_style(Style::default().bg(theme::BG));
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
        RecoveryStage::Checking => {
            let msg = "Checking recovery status...";
            let x = left + (inner_w.saturating_sub(msg.len())) as u16 / 2;
            let y = popup.y + popup.height / 2;
            write_str(
                buf,
                &bounds,
                x,
                y,
                msg,
                Style::default()
                    .fg(theme::CYAN)
                    .bg(theme::BG)
                    .add_modifier(Modifier::SLOW_BLINK),
            );

            let hint = "Esc close";
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
        RecoveryStage::Setup => {
            let msg = "No recovery key found";
            let x = left + (inner_w.saturating_sub(msg.len())) as u16 / 2;
            let y = popup.y + popup.height / 2 - 1;
            write_str(
                buf,
                &bounds,
                x,
                y,
                msg,
                Style::default().fg(theme::TEXT).bg(theme::BG),
            );

            let msg2 = "Press Enter to create one";
            let x2 = left + (inner_w.saturating_sub(msg2.len())) as u16 / 2;
            write_str(
                buf,
                &bounds,
                x2,
                y + 2,
                msg2,
                Style::default()
                    .fg(theme::GREEN)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            );

            let hint = "Enter create \u{00b7} Esc close";
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
        RecoveryStage::Creating => {
            let msg = "Creating recovery key...";
            let x = left + (inner_w.saturating_sub(msg.len())) as u16 / 2;
            let y = popup.y + popup.height / 2;
            write_str(
                buf,
                &bounds,
                x,
                y,
                msg,
                Style::default()
                    .fg(theme::CYAN)
                    .bg(theme::BG)
                    .add_modifier(Modifier::SLOW_BLINK),
            );
        }
        RecoveryStage::ShowKey(key) => {
            let label = "Recovery key:";
            let lx = left + (inner_w.saturating_sub(label.len())) as u16 / 2;
            let y = popup.y + 3;
            write_str(
                buf,
                &bounds,
                lx,
                y,
                label,
                Style::default().fg(theme::TEXT).bg(theme::BG),
            );

            // Display key in cyan/bold, centered
            let key_display = truncate_str(key, inner_w);
            let kx = left + (inner_w.saturating_sub(key_display.len())) as u16 / 2;
            write_str(
                buf,
                &bounds,
                kx,
                y + 2,
                &key_display,
                Style::default()
                    .fg(theme::CYAN)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            );

            let warn = "Save this key somewhere safe!";
            let wx = left + (inner_w.saturating_sub(warn.len())) as u16 / 2;
            write_str(
                buf,
                &bounds,
                wx,
                y + 4,
                warn,
                Style::default()
                    .fg(theme::RED)
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
        RecoveryStage::Enabled => {
            let msg = "Recovery is enabled \u{2713}";
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

            let hint = "r reset \u{00b7} Esc close";
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
        RecoveryStage::Resetting => {
            let msg = "Resetting recovery key...";
            let x = left + (inner_w.saturating_sub(msg.len())) as u16 / 2;
            let y = popup.y + popup.height / 2;
            write_str(
                buf,
                &bounds,
                x,
                y,
                msg,
                Style::default()
                    .fg(theme::CYAN)
                    .bg(theme::BG)
                    .add_modifier(Modifier::SLOW_BLINK),
            );
        }
        RecoveryStage::Failed(err) => {
            let label = "Error:";
            let lx = left + (inner_w.saturating_sub(label.len())) as u16 / 2;
            let y = popup.y + popup.height / 2 - 1;
            write_str(
                buf,
                &bounds,
                lx,
                y,
                label,
                Style::default()
                    .fg(theme::RED)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD),
            );

            let err_display = truncate_str(err, inner_w);
            let ex = left + (inner_w.saturating_sub(err_display.len())) as u16 / 2;
            write_str(
                buf,
                &bounds,
                ex,
                y + 2,
                &err_display,
                Style::default().fg(theme::DIM).bg(theme::BG),
            );

            let hint = "Esc close";
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

    // Decorative glyph
    let gx = x2.saturating_sub(5);
    if gx > x1 {
        set_cell(buf, bounds, gx, y2, '\u{25c8}', s);
    }
}

fn render_title(buf: &mut Buffer, bounds: &Rect, area: Rect, color: Color) {
    let title = "RECOVERY";
    let border_s = Style::default().fg(color).bg(theme::BG);
    let title_s = border_s.add_modifier(Modifier::BOLD);

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
