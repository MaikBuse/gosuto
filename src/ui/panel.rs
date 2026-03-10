use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders};

use crate::ui::{gradient, theme};

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

/// Box-drawing characters used by ratatui's default `Borders::ALL`.
const BOX_CHARS: &[char] = &['┌', '┐', '└', '┘', '─', '│'];

fn is_box_char(ch: char) -> bool {
    BOX_CHARS.contains(&ch)
}

/// Overwrite border-character cells with a gradient fg color (cyan→magenta)
/// walking clockwise around the perimeter. Skips cells that are not
/// box-drawing characters (i.e., title text).
pub fn apply_gradient_border(buf: &mut Buffer, area: Rect, start: Color, end: Color, phase: f32) {
    let bounds = *buf.area();
    gradient::walk_perimeter(area, |x, y, i, total| {
        if x < bounds.x
            || x >= bounds.x + bounds.width
            || y < bounds.y
            || y >= bounds.y + bounds.height
        {
            return;
        }
        let cell = &mut buf[(x, y)];
        let ch = cell.symbol().chars().next().unwrap_or(' ');
        if !is_box_char(ch) {
            return;
        }
        let color = gradient::perimeter_color(i, total, start, end, phase);
        cell.set_style(Style::default().fg(color).bg(theme::BG));
    });
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
