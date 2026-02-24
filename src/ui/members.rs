use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::app::App;
use crate::input::FocusPanel;
use crate::ui::theme;

/// IRC-style power level prefix
fn power_prefix(power_level: i64) -> &'static str {
    match power_level {
        100 => "~", // owner
        75..=99 => "&",  // admin
        50..=74 => "@",  // mod/op
        1..=49 => "+",   // voice
        _ => " ",        // regular
    }
}

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.vim.focus == FocusPanel::Members;

    let border_style = if focused {
        theme::border_focused_style()
    } else {
        theme::border_style()
    };

    let member_count = app.members_list.members.len();
    let title = format!(" MEMBERS ({}) ", member_count);

    let block = Block::default()
        .title(Line::from(vec![Span::styled(title, theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(ratatui::style::Style::default().bg(theme::BG));

    let items: Vec<ListItem> = app
        .members_list
        .members
        .iter()
        .skip(app.members_list.scroll_offset)
        .map(|member| {
            let prefix = power_prefix(member.power_level);
            let prefix_style = if member.power_level >= 50 {
                ratatui::style::Style::default().fg(theme::GREEN)
            } else if member.power_level > 0 {
                ratatui::style::Style::default().fg(theme::CYAN)
            } else {
                theme::dim_style()
            };

            let name_color = theme::sender_color(&member.user_id);
            let name_style = ratatui::style::Style::default().fg(name_color);

            ListItem::new(Line::from(vec![
                Span::styled(prefix, prefix_style),
                Span::styled(&member.display_name, name_style),
            ]))
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
