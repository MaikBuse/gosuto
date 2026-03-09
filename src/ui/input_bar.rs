use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
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
        Style::default().fg(style)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border_style())
        .style(Style::default().bg(theme::BG));

    let prefix = match app.vim.mode {
        VimMode::Insert => "> ",
        VimMode::Command => "",
        VimMode::Normal if app.vim.searching => "",
        VimMode::Normal => "> ",
    };

    let text_lines: Vec<Line> =
        if app.vim.mode == VimMode::Insert && !app.vim.input_buffer.is_empty() {
            content
                .split('\n')
                .enumerate()
                .map(|(i, line_str)| {
                    let line_prefix = if i == 0 { prefix } else { "  " };
                    Line::from(vec![
                        Span::styled(line_prefix, Style::default().fg(style)),
                        Span::styled(line_str.to_string(), text_style),
                    ])
                })
                .collect()
        } else {
            vec![Line::from(vec![
                Span::styled(prefix, Style::default().fg(style)),
                Span::styled(content, text_style),
            ])]
        };

    let paragraph = Paragraph::new(text_lines).block(block);

    frame.render_widget(paragraph, area);

    // Show cursor in insert/command mode
    if app.vim.mode == VimMode::Insert || app.vim.mode == VimMode::Command || app.vim.searching {
        let (cursor_x, cursor_y) = match app.vim.mode {
            VimMode::Insert => {
                let (row, col) = app.vim.cursor_row_col();
                let line_prefix_len = if row == 0 { prefix.len() } else { 2 }; // "  " for continuation
                let x = area.x + 1 + line_prefix_len as u16 + col as u16;
                let y = area.y + 1 + row as u16;
                (x, y)
            }
            VimMode::Command => {
                let x = area.x + 1 + prefix.len() as u16 + app.vim.command_buffer.len() as u16 + 1;
                let y = area.y + 1;
                (x, y)
            }
            VimMode::Normal => {
                let x = area.x + 1 + prefix.len() as u16 + app.vim.search_query.len() as u16 + 1;
                let y = area.y + 1;
                (x, y)
            }
        };
        frame.set_cursor_position((cursor_x, cursor_y));
    }
}
