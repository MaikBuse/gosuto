pub mod call_overlay;
pub mod chat;
pub mod input_bar;
pub mod layout;
pub mod login;
pub mod members;
pub mod room_list;
pub mod status_bar;
pub mod theme;

use ratatui::Frame;

use crate::app::App;
use crate::voip::CallState;

pub fn render(app: &App, frame: &mut Frame) {
    if !app.auth.is_logged_in() {
        login::render(&app.login, &app.auth, frame);
        return;
    }

    let layout = layout::compute_layout(frame);

    room_list::render(app, frame, layout.room_list);
    chat::render(app, frame, layout.chat_area);
    input_bar::render(app, frame, layout.input_bar);
    members::render(app, frame, layout.members_list);
    status_bar::render(app, frame, layout.status_bar);

    // Render call overlay on top if there's an incoming call ringing
    if let Some(ref info) = app.call_info {
        if info.state == CallState::Ringing {
            call_overlay::render(info, frame);
        }
    }
}
