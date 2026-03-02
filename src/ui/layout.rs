use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
};

pub struct AppLayout {
    pub room_list: Rect,
    pub chat_area: Rect,
    pub input_bar: Rect,
    pub members_list: Rect,
    pub status_bar: Rect,
}

pub fn compute_layout(frame: &Frame, input_lines: usize) -> AppLayout {
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
            Constraint::Length(24), // room list
            Constraint::Min(30),    // chat area
            Constraint::Length(20), // members list
        ])
        .split(content);

    let room_list = horizontal[0];
    let middle_panel = horizontal[1];
    let members_list = horizontal[2];

    // Middle panel vertical split: chat | input bar
    let input_height = (input_lines + 2).min(12) as u16; // +2 for borders, cap at 12
    let middle_vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),               // chat messages
            Constraint::Length(input_height), // input bar
        ])
        .split(middle_panel);

    let chat_area = middle_vertical[0];
    let input_bar = middle_vertical[1];

    AppLayout {
        room_list,
        chat_area,
        input_bar,
        members_list,
        status_bar,
    }
}
