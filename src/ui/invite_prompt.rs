use ratatui::Frame;
use ratatui::style::Style;

use crate::app::App;
use crate::ui::popup;
use crate::ui::theme;

const POPUP_WIDTH: u16 = 40;
const POPUP_HEIGHT: u16 = 8;

pub fn render(app: &App, frame: &mut Frame) {
    let Some(ref room_id) = app.invite_prompt_room else {
        return;
    };

    let area = frame.area();
    if area.width < 30 || area.height < 10 {
        return;
    }

    let room_name = app
        .room_list
        .rooms
        .iter()
        .find(|r| r.id == *room_id)
        .map(|r| r.name.as_str())
        .unwrap_or(room_id);

    let popup_w = POPUP_WIDTH.min(area.width.saturating_sub(4));
    let popup_h = POPUP_HEIGHT.min(area.height.saturating_sub(4));
    let popup_area = popup::centered_rect(popup_w, popup_h, area);
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    popup::render_popup_chrome(
        buf,
        &bounds,
        popup_area,
        "INVITATION",
        app.room_list_anim.pulse_phase,
    );

    let left = popup_area.x + 3;
    let inner_w = popup_area.width.saturating_sub(6) as usize;
    let text_style = Style::default().fg(theme::TEXT).bg(theme::BG);

    let display_name = popup::truncate_str(room_name, inner_w);
    let y = popup_area.y + 3;
    let x = left + (inner_w.saturating_sub(display_name.chars().count())) as u16 / 2;
    popup::write_str(buf, &bounds, x, y, &display_name, text_style);

    popup::render_hint(
        buf,
        &bounds,
        popup_area,
        "Enter accept \u{00b7} d decline \u{00b7} Esc close",
    );
}
