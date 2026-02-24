use ratatui::{
    Frame,
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::app::App;
use crate::input::VimMode;
use crate::ui::theme;
use crate::voip::CallState;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let mode_style = match app.vim.mode {
        VimMode::Normal => theme::normal_mode_style(),
        VimMode::Insert => theme::insert_mode_style(),
        VimMode::Command => theme::command_mode_style(),
    };

    let mode_label = format!(" {} ", app.vim.mode);

    let room_name = app
        .room_list
        .selected_room()
        .map(|r| format!(" {} ", r.name))
        .unwrap_or_default();

    let sync_status = format!(" {} ", app.sync_status);

    // Call status
    let call_span = if let Some(ref info) = app.call_info {
        match info.state {
            CallState::Ringing => Span::styled(
                format!(" INCOMING: {} ", info.remote_user),
                ratatui::style::Style::default()
                    .fg(theme::GREEN)
                    .add_modifier(Modifier::BOLD | Modifier::SLOW_BLINK),
            ),
            CallState::Inviting => Span::styled(
                format!(" CALLING: {} ", info.remote_user),
                ratatui::style::Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            CallState::Connecting => Span::styled(
                format!(" CONNECTING: {} ", info.remote_user),
                ratatui::style::Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD),
            ),
            CallState::Active => Span::styled(
                format!(" CALL {} {} ", info.elapsed_display(), info.remote_user),
                ratatui::style::Style::default()
                    .fg(theme::GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
        }
    } else {
        Span::raw("")
    };

    let error_span = if let Some(ref err) = app.last_error {
        Span::styled(format!(" {} ", err), theme::error_style())
    } else {
        Span::raw("")
    };

    let line = Line::from(vec![
        Span::styled(mode_label, mode_style),
        Span::styled(" \u{2502} ", theme::dim_style()), // │
        Span::styled(room_name, theme::text_style()),
        Span::styled(" \u{2502} ", theme::dim_style()),
        Span::styled(sync_status, theme::dim_style()),
        call_span,
        error_span,
    ]);

    let bar = Paragraph::new(line)
        .style(ratatui::style::Style::default().bg(theme::BG));

    frame.render_widget(bar, area);
}
