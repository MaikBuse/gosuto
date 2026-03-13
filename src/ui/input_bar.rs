use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;
use crate::input::{FocusPanel, VimMode};
use crate::ui::{panel, theme};

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
            } else if app.vim.focus == FocusPanel::Messages {
                (
                    "j/k: navigate, r: reply, e: edit, a: react, d: delete".to_string(),
                    theme::DIM,
                )
            } else if app.vim.focus == FocusPanel::Members {
                ("Enter: dm, c: call, v: verify".to_string(), theme::DIM)
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
        .style(Style::default().bg(theme::CHAT_BG));

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

    let mut all_lines = Vec::new();
    if let Some(ref ctx) = app.reply_context {
        all_lines.push(Line::from(vec![
            Span::styled(" Replying to ", theme::reply_indicator_style()),
            Span::styled(
                &ctx.sender,
                Style::default()
                    .fg(theme::sender_color(&ctx.sender))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  {}", ctx.body_preview), theme::dim_style()),
        ]));
    } else if let Some(ref ctx) = app.edit_context {
        let preview = crate::app::truncate_preview(&ctx.original_body, 50);
        all_lines.push(Line::from(vec![
            Span::styled(" Editing ", theme::edit_indicator_style()),
            Span::styled(format!(" {}", preview), theme::dim_style()),
        ]));
    }
    all_lines.extend(text_lines);

    let paragraph = Paragraph::new(all_lines).block(block);

    frame.render_widget(paragraph, area);

    // Gradient border in Insert/Command mode
    let phase = app.anim_clock.phase;
    match app.vim.mode {
        VimMode::Insert => {
            panel::apply_gradient_border_with_bg(
                frame.buffer_mut(),
                area,
                theme::GREEN,
                theme::INPUT_BORDER_GREEN_DIM,
                phase,
                theme::CHAT_BG,
            );
        }
        VimMode::Command => {
            panel::apply_gradient_border_with_bg(
                frame.buffer_mut(),
                area,
                theme::MAGENTA,
                theme::INPUT_BORDER_MAGENTA_DIM,
                phase,
                theme::CHAT_BG,
            );
        }
        VimMode::Normal if app.vim.searching => {
            panel::apply_gradient_border_with_bg(
                frame.buffer_mut(),
                area,
                theme::CYAN,
                theme::GRADIENT_BORDER_END,
                phase,
                theme::CHAT_BG,
            );
        }
        VimMode::Normal => {}
    }

    // Show cursor in insert/command mode
    if app.vim.mode == VimMode::Insert || app.vim.mode == VimMode::Command || app.vim.searching {
        let reply_offset: u16 = if app.reply_context.is_some() || app.edit_context.is_some() {
            1
        } else {
            0
        };
        let (cursor_x, cursor_y) = match app.vim.mode {
            VimMode::Insert => {
                let (row, col) = app.vim.cursor_row_col();
                let line_prefix_len = if row == 0 { prefix.len() } else { 2 }; // "  " for continuation
                let x = area.x + 1 + line_prefix_len as u16 + col as u16;
                let y = area.y + 1 + reply_offset + row as u16;
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
        if app.anim_clock.cursor_visible() {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}
