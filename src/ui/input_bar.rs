use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::input::{FocusPanel, VimMode};
use crate::ui::theme;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let (content, style) = match app.vim.mode {
        VimMode::Command => {
            let cmd = format!(":{}", app.vim.command_buffer);
            (cmd, theme::MAGENTA)
        }
        VimMode::Insert => {
            let text = if app.vim.input_buffer.is_empty() {
                "type message here...".to_string()
            } else {
                app.vim.input_buffer.clone()
            };
            (text, theme::GREEN)
        }
        VimMode::Normal => {
            if app.vim.searching {
                let search = format!("/{}", app.vim.search_query);
                (search, theme::CYAN)
            } else if app.vim.focus == FocusPanel::Members {
                ("Enter: dm, c: call".to_string(), theme::DIM)
            } else {
                ("press i to type, : for commands".to_string(), theme::DIM)
            }
        }
    };

    let is_placeholder = app.vim.mode == VimMode::Normal && !app.vim.searching
        || (app.vim.mode == VimMode::Insert && app.vim.input_buffer.is_empty());

    let text_style = if is_placeholder {
        theme::dim_style()
    } else {
        ratatui::style::Style::default().fg(style)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border_style())
        .style(ratatui::style::Style::default().bg(theme::BG));

    let prefix = match app.vim.mode {
        VimMode::Insert => "> ",
        VimMode::Command => "",
        VimMode::Normal if app.vim.searching => "",
        VimMode::Normal => "> ",
    };

    let paragraph = Paragraph::new(Line::from(vec![
        Span::styled(prefix, ratatui::style::Style::default().fg(style)),
        Span::styled(content, text_style),
    ]))
    .block(block);

    frame.render_widget(paragraph, area);

    // Show cursor in insert/command mode
    if app.vim.mode == VimMode::Insert || app.vim.mode == VimMode::Command || app.vim.searching {
        let cursor_x = area.x
            + 1 // border
            + prefix.len() as u16
            + match app.vim.mode {
                VimMode::Insert => app.vim.input_cursor as u16,
                VimMode::Command => app.vim.command_buffer.len() as u16,
                VimMode::Normal => app.vim.search_query.len() as u16,
            };
        let cursor_y = area.y + 1; // border
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}
