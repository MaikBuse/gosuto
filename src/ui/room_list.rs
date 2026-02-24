use ratatui::{
    Frame,
    layout::Rect,
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::app::App;
use crate::input::FocusPanel;
use crate::state::RoomCategory;
use crate::ui::theme;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.vim.focus == FocusPanel::RoomList;

    let border_style = if focused {
        theme::border_focused_style()
    } else {
        theme::border_style()
    };

    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" ROOMS ", theme::title_style()),
        ]))
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(ratatui::style::Style::default().bg(theme::BG));

    let visible = app.room_list.visible_rooms();
    let items: Vec<ListItem> = visible
        .iter()
        .map(|(vi, room)| {
            let is_selected = *vi == app.room_list.selected;
            let prefix = match room.category {
                RoomCategory::Space => "\u{2261} ", // ≡
                RoomCategory::Room => "# ",
                RoomCategory::DirectMessage => "@ ",
            };

            let label = format!("{}{}", prefix, room.name);
            let style = if is_selected {
                theme::selected_style()
            } else {
                match room.category {
                    RoomCategory::Space => theme::dim_style().add_modifier(Modifier::BOLD),
                    RoomCategory::Room => theme::text_style(),
                    RoomCategory::DirectMessage => theme::text_style(),
                }
            };

            let mut spans = vec![Span::styled(label, style)];

            if room.unread_count > 0 && !is_selected {
                spans.push(Span::styled(
                    format!(" ({})", room.unread_count),
                    ratatui::style::Style::default().fg(theme::CYAN),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items).block(block);

    frame.render_widget(list, area);
}
