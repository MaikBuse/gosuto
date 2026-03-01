pub mod audio_settings;
pub mod call_overlay;
pub mod chat;
pub mod completion_popup;
pub mod effects;
pub mod input_bar;
pub mod layout;
pub mod login;
pub mod members;
pub mod room_info;
pub mod room_list;
pub mod status_bar;
pub mod theme;
pub mod verify_modal;

use ratatui::Frame;

use crate::app::App;
pub fn render(app: &App, frame: &mut Frame) {
    if !app.auth.is_logged_in() {
        login::render(&app.login, &app.auth, frame);
        let area = frame.area();
        if app.effects.enabled
            && let Some(effect_buf) = app.effects.render_to_buffer(area)
        {
            effects::composite(frame.buffer_mut(), &effect_buf, area);
        }
        app.effects.post_process_glitch(frame.buffer_mut(), &[area]);
        return;
    }

    let layout = layout::compute_layout(frame);

    room_list::render(app, frame, layout.room_list);
    chat::render(app, frame, layout.chat_area);
    input_bar::render(app, frame, layout.input_bar);
    members::render(app, frame, layout.members_list);
    status_bar::render(app, frame, layout.status_bar);

    // Composite effects behind UI in content panels only (before overlays)
    if app.effects.enabled {
        // Room list: EMP pulse effect
        let scroll_off = room_list::scroll_offset(app, layout.room_list);
        if let Some(emp_buf) = app.effects.render_emp_buffer(layout.room_list, scroll_off) {
            effects::composite(frame.buffer_mut(), &emp_buf, layout.room_list);
        }

        // Members list: EMP pulse effect
        let members_scroll_off = members::scroll_offset(app, layout.members_list);
        if let Some(emp_buf) =
            app.effects
                .render_members_emp_buffer(layout.members_list, members_scroll_off)
        {
            effects::composite(frame.buffer_mut(), &emp_buf, layout.members_list);
        }

        // Chat: matrix rain (room list + members use EMP instead)
        if let Some(effect_buf) = app.effects.render_to_buffer(frame.area()) {
            effects::composite(frame.buffer_mut(), &effect_buf, layout.chat_area);
        }
    }

    // Glitch post-processing: displace content bands with chromatic aberration
    let content_panels = [layout.room_list, layout.chat_area, layout.members_list];
    app.effects
        .post_process_glitch(frame.buffer_mut(), &content_panels);

    // Command auto-completion popup (rendered after effects so it isn't overwritten)
    completion_popup::render(app, frame, layout.input_bar);

    // Render call overlay on top for any active call state or incoming ringing
    if let Some(ref info) = app.call_info {
        let ds = if info.is_incoming
            && info.state == crate::voip::CallState::Connecting
            && info.started_at.is_none()
        {
            call_overlay::CallDisplayState::Connecting
        } else {
            match info.state {
                crate::voip::CallState::Connecting => call_overlay::CallDisplayState::Connecting,
                crate::voip::CallState::Active => call_overlay::CallDisplayState::Active,
            }
        };
        call_overlay::render(&app.call_popup, info, &ds, frame);
    } else if let Some(ref room_id) = app.incoming_call_room {
        let caller = app.incoming_call_user.as_deref().unwrap_or("unknown");
        call_overlay::render_ringing(
            &app.call_popup,
            caller,
            room_id,
            app.incoming_call_room_name.as_deref(),
            frame,
        );
    }

    // Audio settings modal overlay
    if app.audio_settings.open {
        audio_settings::render(&app.audio_settings, frame);
    }

    // Room info modal overlay
    if app.room_info.open {
        room_info::render(&app.room_info, frame);
    }

    // Verification modal overlay
    if let Some(ref verify_state) = app.verification_modal {
        verify_modal::render(verify_state, frame);
    }
}
