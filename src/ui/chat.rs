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
        .messages
        .current_room_id
        .as_ref()
        .and_then(|id| app.room_list.rooms.iter().find(|r| r.id == *id))
        .map(|r| r.name.clone())
        .unwrap_or_else(|| "No room selected".to_string());

    let title_text = format!(" > {} ", room_name);
    let title_line = app
        .chat_title_reveal
        .render_line(&title_text, theme::title_style());

    let block = Block::default()
        .title(title_line)
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(ratatui::style::Style::default().bg(theme::BG));

    let inner_height = area.height.saturating_sub(2) as usize; // borders
    let inner_width = area.width.saturating_sub(2) as usize; // borders

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

    let mut lines: Vec<Line> = Vec::new();
    let mut last_date: Option<chrono::NaiveDate> = None;

    for msg in messages {
        let msg_date = msg.timestamp.date_naive();
        if last_date != Some(msg_date) {
            let date_str = msg.timestamp.format("%B %-d, %Y").to_string();
            let sep = format!("─── {} ───", date_str);
            lines.push(Line::from(Span::styled(sep, theme::dim_style())));
            last_date = Some(msg_date);
        }

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

        lines.push(Line::from(spans));
    }

    // Compute total visual lines accounting for wrapping
    let total_visual_lines: usize = if inner_width > 0 {
        lines
            .iter()
            .map(|line| {
                let w = line.width();
                if w == 0 { 1 } else { w.div_ceil(inner_width) }
            })
            .sum()
    } else {
        lines.len()
    };

    let max_scroll = total_visual_lines.saturating_sub(inner_height);
    let clamped_offset = app.messages.scroll_offset.min(max_scroll);
    let scroll_y = max_scroll.saturating_sub(clamped_offset);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_y as u16, 0));

    frame.render_widget(paragraph, area);
}
