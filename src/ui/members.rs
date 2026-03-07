use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::app::App;
use crate::input::FocusPanel;
use crate::ui::icons::Icons;
use crate::ui::theme;

/// IRC-style power level prefix
fn power_prefix(power_level: i64, icons: &Icons) -> &str {
    match power_level {
        100 => icons.power_owner,
        75..=99 => icons.power_admin,
        50..=74 => icons.power_mod,
        1..=49 => icons.power_voice,
        _ => icons.power_none,
    }
}

pub fn scroll_offset(app: &App, area: Rect) -> usize {
    let inner_height = area.height.saturating_sub(2) as usize; // subtract borders
    let total_rows = app.members_list.members.len();
    let selected = app.members_list.selected;

    if total_rows <= inner_height || selected < inner_height / 2 {
        0
    } else if selected > total_rows - inner_height / 2 {
        total_rows - inner_height
    } else {
        selected - inner_height / 2
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

    let title_line = app
        .members_title_reveal
        .render_line(&title, theme::title_style());

    let block = Block::default()
        .title(title_line)
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(ratatui::style::Style::default().bg(theme::BG));

    let icons = app.config.icons();

    let items: Vec<ListItem> = app
        .members_list
        .members
        .iter()
        .map(|member| {
            let prefix = power_prefix(member.power_level, icons);
            let prefix_style = if member.power_level >= 50 {
                ratatui::style::Style::default().fg(theme::GREEN)
            } else if member.power_level > 0 {
                ratatui::style::Style::default().fg(theme::CYAN)
            } else {
                theme::dim_style()
            };

            let name_color = theme::sender_color(&member.user_id);
            let name_style = ratatui::style::Style::default().fg(name_color);

            let mut spans = vec![
                Span::styled(prefix, prefix_style),
                Span::styled(&member.display_name, name_style),
            ];

            if member.verified == Some(true) {
                spans.push(Span::styled(
                    format!(" {}", icons.checkmark),
                    ratatui::style::Style::default().fg(theme::GREEN),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let highlight_style = ratatui::style::Style::default()
        .fg(theme::CYAN)
        .bg(ratatui::style::Color::Rgb(20, 20, 40));

    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style);

    let mut list_state = ListState::default();
    if focused && !app.members_list.members.is_empty() {
        list_state.select(Some(app.members_list.selected));
    }

    frame.render_stateful_widget(list, area, &mut list_state);
}
