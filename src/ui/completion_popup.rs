use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
};

use crate::app::App;
use crate::input::VimMode;
use crate::input::command::filtered_commands;
use crate::ui::theme;

pub fn render(app: &App, frame: &mut Frame, input_bar_area: Rect) {
    // Only show in Command mode, before any argument (no space in buffer)
    if app.vim.mode != VimMode::Command || app.vim.command_buffer.contains(' ') {
        return;
    }

    let matches = filtered_commands(&app.vim.command_buffer);
    if matches.is_empty() {
        return;
    }

    let max_height: u16 = 14; // cap popup height
    let item_count = matches.len() as u16;
    let popup_height = item_count.min(max_height) + 2; // +2 for borders

    // Position popup above the input bar
    let popup_y = input_bar_area.y.saturating_sub(popup_height);
    let popup_area = Rect::new(
        input_bar_area.x,
        popup_y,
        input_bar_area.width / 2,
        popup_height,
    );

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    let items: Vec<ListItem> = matches
        .iter()
        .enumerate()
        .map(|(i, cmd)| {
            let is_selected = app.vim.completion.selected == Some(i);

            let syntax_span = Span::styled(
                format!(" {:<18}", cmd.syntax),
                if is_selected {
                    Style::default()
                        .fg(theme::BLACK)
                        .bg(theme::MAGENTA)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::MAGENTA)
                },
            );

            let desc_span = Span::styled(
                cmd.description,
                if is_selected {
                    Style::default()
                        .fg(theme::BLACK)
                        .bg(theme::MAGENTA)
                } else {
                    Style::default().fg(theme::DIM)
                },
            );

            let bg_span = if is_selected {
                Span::styled(" ", Style::default().bg(theme::MAGENTA))
            } else {
                Span::raw("")
            };

            ListItem::new(Line::from(vec![syntax_span, desc_span, bg_span]))
        })
        .collect();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::MAGENTA))
        .style(Style::default().bg(theme::BG));

    let list = List::new(items).block(block);
    frame.render_widget(list, popup_area);
}
