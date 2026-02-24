use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::ui::theme;
use crate::voip::CallInfo;

/// Render a centered call overlay popup for incoming calls
pub fn render(info: &CallInfo, frame: &mut Frame) {
    let area = frame.area();

    // Calculate centered popup area (40x7)
    let popup_width = 42u16.min(area.width.saturating_sub(4));
    let popup_height = 7u16.min(area.height.saturating_sub(4));

    let popup_area = centered_rect(popup_width, popup_height, area);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::GREEN))
        .title(" Incoming Call ")
        .title_alignment(Alignment::Center)
        .style(Style::default().bg(theme::BG));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Content lines
    let caller_line = Line::from(vec![
        Span::styled(
            &info.remote_user,
            Style::default()
                .fg(theme::GREEN)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let hint_line = Line::from(vec![
        Span::styled(":answer", Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD)),
        Span::styled(" | ", Style::default().fg(theme::DIM)),
        Span::styled(":reject", Style::default().fg(theme::RED).add_modifier(Modifier::BOLD)),
    ]);

    let content = vec![
        Line::from(""),
        caller_line,
        Line::from(""),
        hint_line,
    ];

    let paragraph = Paragraph::new(content)
        .alignment(Alignment::Center)
        .style(Style::default().fg(theme::TEXT).bg(theme::BG));

    frame.render_widget(paragraph, inner);
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((area.height.saturating_sub(height)) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(area);

    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length((area.width.saturating_sub(width)) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(vertical[1]);

    horizontal[1]
}
