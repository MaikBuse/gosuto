pub mod audio_settings;
pub mod call_overlay;
pub mod chat;
pub mod completion_popup;
pub mod effects;
pub mod input_bar;
pub mod layout;
pub mod login;
pub mod members;
pub mod room_list;
pub mod status_bar;
pub mod theme;

use ratatui::Frame;

use crate::app::App;
pub fn render(app: &App, frame: &mut Frame) {
    if !app.auth.is_logged_in() {
        login::render(&app.login, &app.auth, frame);
        let area = frame.area();
        if app.effects.enabled {
            if let Some(effect_buf) = app.effects.render_to_buffer(area) {
                effects::composite(frame.buffer_mut(), &effect_buf, area);
            }
        }
        app.effects
            .post_process_glitch(frame.buffer_mut(), &[area]);
        return;
    }

    let layout = layout::compute_layout(frame);

    room_list::render(app, frame, layout.room_list);
    chat::render(app, frame, layout.chat_area);
    input_bar::render(app, frame, layout.input_bar);
    members::render(app, frame, layout.members_list);
    status_bar::render(app, frame, layout.status_bar);

    // Composite effects behind UI in content panels only (before overlays)
    if app.effects.enabled
        && let Some(effect_buf) = app.effects.render_to_buffer(frame.area())
    {
        effects::composite(frame.buffer_mut(), &effect_buf, layout.room_list);
        effects::composite(frame.buffer_mut(), &effect_buf, layout.chat_area);
        effects::composite(frame.buffer_mut(), &effect_buf, layout.members_list);
    }

    // Glitch post-processing: displace content bands with chromatic aberration
    let content_panels = [layout.room_list, layout.chat_area, layout.members_list];
    app.effects
        .post_process_glitch(frame.buffer_mut(), &content_panels);

    // Command auto-completion popup (rendered after effects so it isn't overwritten)
    completion_popup::render(app, frame, layout.input_bar);

    // Render call overlay on top for any active call state
    if let Some(ref info) = app.call_info {
        call_overlay::render(&app.call_popup, info, frame);
    }

    // Audio settings modal overlay
    if app.audio_settings.open {
        audio_settings::render(&app.audio_settings, frame);
    }
}
