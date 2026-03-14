pub mod animation;
pub mod audio_settings;
pub mod call_overlay;
pub mod change_password;
pub mod chat;
pub mod completion_popup;
pub mod configure;
pub mod create_room;
pub mod effects;
pub mod emoji_data;
pub mod form_field;
pub mod gradient;
pub mod icons;
pub mod input_bar;
pub mod invite_prompt;
pub mod layout;
pub mod login;
pub mod members;
pub mod panel;
pub mod popup;
pub mod reaction_picker;
pub mod recovery_modal;
pub mod redact_confirm;
pub mod rich_text;
pub mod room_info;
pub mod room_list;
pub mod status_bar;
pub mod theme;
pub mod tooltip;
pub mod typing_indicator;
pub mod verify_modal;
pub mod which_key;

use ratatui::Frame;

use crate::app::App;
pub fn render(app: &mut App, frame: &mut Frame) {
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

    let has_typing = app
        .messages
        .current_room_id
        .as_ref()
        .and_then(|rid| app.typing_users.get(rid))
        .is_some_and(|names| !names.is_empty());
    let reply_extra = if app.reply_context.is_some() || app.edit_context.is_some() {
        1
    } else {
        0
    };
    let text_width = frame.area().width.saturating_sub(32 + 32 + 2 + 2); // rooms + members + borders + prefix
    let input_lines = app.vim.visual_line_count(text_width) + reply_extra;
    let layout = layout::compute_layout(frame, input_lines, has_typing);

    room_list::render(app, frame, layout.room_list);
    chat::render(app, frame, layout.chat_area);
    if let Some(typing_area) = layout.typing_indicator {
        typing_indicator::render(app, frame, typing_area);
    }
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
        if let Some(emp_buf) = app
            .effects
            .render_members_emp_buffer(layout.members_list, members_scroll_off)
        {
            effects::composite(frame.buffer_mut(), &emp_buf, layout.members_list);
        }
    }

    // Glitch post-processing: displace content bands with chromatic aberration
    let content_panels = [layout.room_list, layout.chat_area, layout.members_list];
    app.effects
        .post_process_glitch(frame.buffer_mut(), &content_panels);

    // Command auto-completion popup (rendered after effects so it isn't overwritten)
    completion_popup::render(app, frame, layout.input_bar);

    // Tooltips for truncated names
    room_list::render_tooltip(app, frame, layout.room_list);
    members::render_tooltip(app, frame, layout.members_list);

    let icons = app.config.icons();
    let border_phase = app.anim_clock.phase;
    let cursor_visible = app.anim_clock.cursor_visible();

    // Render call overlay on top for any active call state or incoming ringing
    if let Some(ref info) = app.call_info {
        let ds = if info.is_incoming
            && matches!(info.state, crate::voip::CallState::Connecting(_))
            && info.started_at.is_none()
        {
            call_overlay::CallDisplayState::Connecting
        } else {
            match info.state {
                crate::voip::CallState::Connecting(_) => call_overlay::CallDisplayState::Connecting,
                crate::voip::CallState::Active => call_overlay::CallDisplayState::Active,
            }
        };
        call_overlay::render(&app.call_popup, info, &ds, icons, frame);
    } else if let Some(ref room_id) = app.incoming_call_room {
        let caller = app.incoming_call_user.as_deref().unwrap_or("unknown");
        call_overlay::render_ringing(
            &app.call_popup,
            caller,
            room_id,
            app.incoming_call_room_name.as_deref(),
            icons,
            frame,
        );
    }

    // Audio settings modal overlay
    if app.audio_settings.open {
        audio_settings::render(&app.audio_settings, icons, frame, border_phase);
    }

    // Room info modal overlay
    if app.room_info.open {
        room_info::render(&app.room_info, icons, frame, border_phase, cursor_visible);
    }

    // Create room modal overlay
    if app.create_room.open {
        create_room::render(&app.create_room, icons, frame, border_phase, cursor_visible);
    }

    // User config modal overlay
    if app.user_config.open {
        configure::render(&app.user_config, icons, frame, border_phase, cursor_visible);
    }

    // Change password modal overlay
    if app.change_password.open {
        change_password::render(
            &app.change_password,
            icons,
            frame,
            border_phase,
            cursor_visible,
        );
    }

    // Recovery modal
    if let Some(ref state) = app.recovery {
        recovery_modal::render(state, frame, border_phase);
    }

    // Verification modal
    if let Some(ref verify_state) = app.verification_modal {
        verify_modal::render(verify_state, frame, border_phase, cursor_visible);
    }

    // Invite prompt
    if app.invite_prompt_room.is_some() {
        invite_prompt::render(app, frame);
    }

    // Reaction picker
    if app.reaction_picker.is_some() {
        reaction_picker::render(app, frame);
    }

    // Redact confirmation
    if app.redact_confirm.is_some() {
        redact_confirm::render(app, frame);
    }

    // Which-key leader popup
    if let Some(ref wk) = app.which_key {
        which_key::render(*wk, app, frame);
    }
}
