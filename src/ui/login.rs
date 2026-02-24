use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::state::AuthState;
use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginField {
    Homeserver,
    Username,
    Password,
}

impl LoginField {
    pub fn next(self) -> Self {
        match self {
            LoginField::Homeserver => LoginField::Username,
            LoginField::Username => LoginField::Password,
            LoginField::Password => LoginField::Homeserver,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            LoginField::Homeserver => LoginField::Password,
            LoginField::Username => LoginField::Homeserver,
            LoginField::Password => LoginField::Username,
        }
    }
}

pub struct LoginState {
    pub homeserver: String,
    pub username: String,
    pub password: String,
    pub focused_field: LoginField,
    pub cursor_pos: usize,
}

impl LoginState {
    pub fn new() -> Self {
        Self {
            homeserver: "https://matrix.org".to_string(),
            username: String::new(),
            password: String::new(),
            focused_field: LoginField::Username,
            cursor_pos: 0,
        }
    }

    pub fn active_buffer(&self) -> &str {
        match self.focused_field {
            LoginField::Homeserver => &self.homeserver,
            LoginField::Username => &self.username,
            LoginField::Password => &self.password,
        }
    }

    pub fn active_buffer_mut(&mut self) -> &mut String {
        match self.focused_field {
            LoginField::Homeserver => &mut self.homeserver,
            LoginField::Username => &mut self.username,
            LoginField::Password => &mut self.password,
        }
    }

    pub fn insert_char(&mut self, c: char) {
        let cursor = self.cursor_pos;
        let buf = self.active_buffer_mut();
        if cursor <= buf.len() {
            buf.insert(cursor, c);
            self.cursor_pos = cursor + c.len_utf8();
        }
    }

    pub fn backspace(&mut self) {
        if self.cursor_pos > 0 {
            let cursor = self.cursor_pos;
            let buf = self.active_buffer_mut();
            let prev = buf[..cursor]
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            buf.remove(prev);
            self.cursor_pos = prev;
        }
    }

    pub fn next_field(&mut self) {
        self.focused_field = self.focused_field.next();
        self.cursor_pos = self.active_buffer().len();
    }

    pub fn prev_field(&mut self) {
        self.focused_field = self.focused_field.prev();
        self.cursor_pos = self.active_buffer().len();
    }
}

pub fn render(login: &LoginState, auth_state: &AuthState, frame: &mut Frame) {
    let area = frame.area();

    // Center the login form
    let form_width = 50u16.min(area.width.saturating_sub(4));
    let form_height = 14u16;
    let form_area = centered_rect(form_width, form_height, area);

    // Clear background
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new("").style(ratatui::style::Style::default().bg(theme::BG)),
        area,
    );

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            " WALRUST ",
            ratatui::style::Style::default()
                .fg(theme::CYAN)
                .add_modifier(Modifier::BOLD),
        )]))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(theme::border_focused_style())
        .style(ratatui::style::Style::default().bg(theme::BG));

    let inner = block.inner(form_area);
    frame.render_widget(block, form_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title spacer
            Constraint::Length(1), // homeserver label
            Constraint::Length(1), // homeserver input
            Constraint::Length(1), // spacer
            Constraint::Length(1), // username label
            Constraint::Length(1), // username input
            Constraint::Length(1), // spacer
            Constraint::Length(1), // password label
            Constraint::Length(1), // password input
            Constraint::Length(1), // spacer
            Constraint::Length(1), // status/error
            Constraint::Min(0),   // remaining
        ])
        .split(inner);

    // Title
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "Matrix Login",
            theme::title_style(),
        )))
        .alignment(Alignment::Center),
        chunks[0],
    );

    // Homeserver
    render_field(
        frame,
        "Homeserver:",
        &login.homeserver,
        false,
        login.focused_field == LoginField::Homeserver,
        chunks[1],
        chunks[2],
    );

    // Username
    render_field(
        frame,
        "Username:",
        &login.username,
        false,
        login.focused_field == LoginField::Username,
        chunks[4],
        chunks[5],
    );

    // Password
    render_field(
        frame,
        "Password:",
        &login.password,
        true,
        login.focused_field == LoginField::Password,
        chunks[7],
        chunks[8],
    );

    // Status/error
    let status = match auth_state {
        AuthState::LoggingIn => Span::styled("Logging in...", theme::dim_style()),
        AuthState::AutoLoggingIn => Span::styled("Auto-logging in...", theme::dim_style()),
        AuthState::Error(e) => Span::styled(e.as_str(), theme::error_style()),
        _ => Span::styled(
            "Tab: next field | Enter: login | Ctrl+C: quit",
            theme::dim_style(),
        ),
    };
    frame.render_widget(
        Paragraph::new(Line::from(status)).alignment(Alignment::Center),
        chunks[10],
    );

    // Cursor
    if !matches!(auth_state, AuthState::LoggingIn | AuthState::AutoLoggingIn) {
        let (cursor_chunk, offset) = match login.focused_field {
            LoginField::Homeserver => (chunks[2], login.cursor_pos),
            LoginField::Username => (chunks[5], login.cursor_pos),
            LoginField::Password => (chunks[8], login.password.len()),
        };
        let cursor_x = cursor_chunk.x + 1 + offset as u16;
        let cursor_y = cursor_chunk.y;
        frame.set_cursor_position((cursor_x.min(cursor_chunk.right() - 1), cursor_y));
    }
}

fn render_field(
    frame: &mut Frame,
    label: &str,
    value: &str,
    is_password: bool,
    focused: bool,
    label_area: Rect,
    input_area: Rect,
) {
    let label_style = if focused {
        ratatui::style::Style::default()
            .fg(theme::CYAN)
            .add_modifier(Modifier::BOLD)
    } else {
        theme::dim_style()
    };

    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(label, label_style))),
        label_area,
    );

    let display_value = if is_password {
        "\u{2022}".repeat(value.len()) // bullet points
    } else {
        value.to_string()
    };

    let input_style = if focused {
        ratatui::style::Style::default().fg(theme::TEXT)
    } else {
        theme::dim_style()
    };

    let prefix = if focused { "\u{25b8} " } else { "  " };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(prefix, ratatui::style::Style::default().fg(theme::CYAN)),
            Span::styled(display_value, input_style),
        ])),
        input_area,
    );
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}
