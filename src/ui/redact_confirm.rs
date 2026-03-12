use ratatui::Frame;
use ratatui::style::Style;

use crate::app::App;
use crate::ui::popup;
use crate::ui::theme;

const POPUP_WIDTH: u16 = 40;
const POPUP_HEIGHT: u16 = 8;

pub fn render(app: &App, frame: &mut Frame) {
    let Some(ref confirm) = app.redact_confirm else {
        return;
    };

    let area = frame.area();
    if area.width < 30 || area.height < 10 {
        return;
    }

    let popup_w = POPUP_WIDTH.min(area.width.saturating_sub(4));
    let popup_h = POPUP_HEIGHT.min(area.height.saturating_sub(4));
    let popup_area = popup::centered_rect(popup_w, popup_h, area);
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    popup::render_popup_chrome(
        buf,
        &bounds,
        popup_area,
        "DELETE MESSAGE",
        app.anim_clock.phase,
    );

    let left = popup_area.x + 3;
    let inner_w = popup_area.width.saturating_sub(6) as usize;
    let text_style = Style::default().fg(theme::TEXT).bg(theme::BG);

    let display_text = popup::truncate_str(&confirm.body_preview, inner_w);
    let y = popup_area.y + 3;
    let x = left + (inner_w.saturating_sub(display_text.chars().count())) as u16 / 2;
    popup::write_str(buf, &bounds, x, y, &display_text, text_style);

    popup::render_hint(buf, &bounds, popup_area, "y confirm \u{00b7} n cancel");
}
