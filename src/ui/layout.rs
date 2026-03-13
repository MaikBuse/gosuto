use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

pub struct AppLayout {
    pub room_list: Rect,
    pub chat_area: Rect,
    pub typing_indicator: Option<Rect>,
    pub input_bar: Rect,
    pub members_list: Rect,
    pub status_bar: Rect,
}

pub fn compute_layout(frame: &Frame, input_lines: usize, show_typing: bool) -> AppLayout {
    let area = frame.area();

    // Main vertical split: content area + status bar
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(5),    // content
            Constraint::Length(1), // status bar
        ])
        .split(area);

    let content = vertical[0];
    let status_bar = vertical[1];

    // Horizontal split: room list | chat+input | members
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(32), // room list
            Constraint::Min(30),    // chat area
            Constraint::Length(32), // members list
        ])
        .split(content);

    let room_list = horizontal[0];
    let middle_panel = horizontal[1];
    let members_list = horizontal[2];

    // Middle panel vertical split: chat | (typing indicator) | input bar
    let input_height = (input_lines + 2).min(12) as u16; // +2 for borders, cap at 12

    let (chat_area, typing_indicator, input_bar) = if show_typing {
        let middle_vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),               // chat messages
                Constraint::Length(1),            // typing indicator
                Constraint::Length(input_height), // input bar
            ])
            .split(middle_panel);
        (
            middle_vertical[0],
            Some(middle_vertical[1]),
            middle_vertical[2],
        )
    } else {
        let middle_vertical = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(3),               // chat messages
                Constraint::Length(input_height), // input bar
            ])
            .split(middle_panel);
        (middle_vertical[0], None, middle_vertical[1])
    };

    AppLayout {
        room_list,
        chat_area,
        typing_indicator,
        input_bar,
        members_list,
        status_bar,
    }
}
