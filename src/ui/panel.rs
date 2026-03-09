use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders};

use crate::ui::theme;

/// Build a standard panel block with consistent border/title styling.
pub fn block<'a>(title: Line<'a>, focused: bool) -> Block<'a> {
    let border_style = if focused {
        theme::border_focused_style()
    } else {
        theme::border_style()
    };

    Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme::BG))
}

/// Compute scroll offset to keep a selected item centered in visible area.
pub fn scroll_offset(total: usize, selected: usize, visible_height: usize) -> usize {
    if total <= visible_height || selected < visible_height / 2 {
        0
    } else if selected > total - visible_height / 2 {
        total - visible_height
    } else {
        selected - visible_height / 2
    }
}
