use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use crate::app::App;
use crate::input::FocusPanel;
use crate::ui::theme;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.vim.focus == FocusPanel::Messages;

    let border_style = if focused {
        theme::border_focused_style()
    } else {
        theme::border_style()
    };

    let room_name = app
        .room_list
        .selected_room()
        .map(|r| r.name.clone())
        .unwrap_or_else(|| "No room selected".to_string());

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            format!(" > {} ", room_name),
            theme::title_style(),
        )]))
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(ratatui::style::Style::default().bg(theme::BG));

    let inner_height = area.height.saturating_sub(2) as usize; // borders

    let messages = &app.messages.messages;
    if messages.is_empty() {
        let placeholder = if app.messages.current_room_id.is_none() {
            Paragraph::new(Line::from(Span::styled(
                "Select a room to start chatting",
                theme::dim_style(),
            )))
        } else if app.messages.loading {
            Paragraph::new(Line::from(Span::styled(
                "Loading messages...",
                theme::dim_style(),
            )))
        } else if let Some(ref err) = app.messages.fetch_error {
            Paragraph::new(Line::from(Span::styled(
                format!("Error: {}", err),
                theme::error_style(),
            )))
        } else {
            Paragraph::new(Line::from(Span::styled(
                "No messages yet",
                theme::dim_style(),
            )))
        };
        frame.render_widget(placeholder.block(block), area);
        return;
    }

    // Calculate visible range with scroll offset
    let total = messages.len();
    let end = total.saturating_sub(app.messages.scroll_offset);
    let start = end.saturating_sub(inner_height);

    let lines: Vec<Line> = messages[start..end]
        .iter()
        .map(|msg| {
            let time = msg.timestamp.format("%H:%M").to_string();
            let sender_color = theme::sender_color(&msg.sender);

            let mut spans = vec![
                Span::styled(format!("{} ", time), theme::dim_style()),
                Span::styled(
                    format!("{} ", msg.sender),
                    ratatui::style::Style::default()
                        .fg(sender_color)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                ),
            ];

            if msg.pending {
                spans.push(Span::styled(&msg.body, theme::dim_style()));
                spans.push(Span::styled(" (sending...)", theme::dim_style()));
            } else if msg.is_emote {
                spans.push(Span::styled(
                    &msg.body,
                    ratatui::style::Style::default().fg(sender_color),
                ));
            } else if msg.is_notice {
                spans.push(Span::styled(&msg.body, theme::dim_style()));
            } else {
                spans.push(Span::styled(&msg.body, theme::text_style()));
            }

            Line::from(spans)
        })
        .collect();

    let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
