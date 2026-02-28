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
pub enum FormMode {
    Login,
    Register,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoginField {
    Homeserver,
    Username,
    Password,
    ConfirmPassword,
    RegistrationToken,
}

pub struct LoginState {
    pub mode: FormMode,
    pub homeserver: String,
    pub username: String,
    pub password: String,
    pub confirm_password: String,
    pub registration_token: String,
    pub focused_field: LoginField,
    pub cursor_pos: usize,
}

impl LoginState {
    pub fn new() -> Self {
        Self {
            mode: FormMode::Login,
            homeserver: "https://matrix.org".to_string(),
            username: String::new(),
            password: String::new(),
            confirm_password: String::new(),
            registration_token: String::new(),
            focused_field: LoginField::Username,
            cursor_pos: 0,
        }
    }

    fn field_order(&self) -> &[LoginField] {
        match self.mode {
            FormMode::Login => &[
                LoginField::Homeserver,
                LoginField::Username,
                LoginField::Password,
            ],
            FormMode::Register => &[
                LoginField::Homeserver,
                LoginField::Username,
                LoginField::Password,
                LoginField::ConfirmPassword,
                LoginField::RegistrationToken,
            ],
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = match self.mode {
            FormMode::Login => FormMode::Register,
            FormMode::Register => {
                // Snap to Password if on a register-only field
                if matches!(
                    self.focused_field,
                    LoginField::ConfirmPassword | LoginField::RegistrationToken
                ) {
                    self.focused_field = LoginField::Password;
                    self.cursor_pos = self.password.len();
                }
                FormMode::Login
            }
        };
    }

    pub fn active_buffer(&self) -> &str {
        match self.focused_field {
            LoginField::Homeserver => &self.homeserver,
            LoginField::Username => &self.username,
            LoginField::Password => &self.password,
            LoginField::ConfirmPassword => &self.confirm_password,
            LoginField::RegistrationToken => &self.registration_token,
        }
    }

    pub fn active_buffer_mut(&mut self) -> &mut String {
        match self.focused_field {
            LoginField::Homeserver => &mut self.homeserver,
            LoginField::Username => &mut self.username,
            LoginField::Password => &mut self.password,
            LoginField::ConfirmPassword => &mut self.confirm_password,
            LoginField::RegistrationToken => &mut self.registration_token,
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
        let order = self.field_order();
        let pos = order
            .iter()
            .position(|f| *f == self.focused_field)
            .unwrap_or(0);
        self.focused_field = order[(pos + 1) % order.len()];
        self.cursor_pos = self.active_buffer().len();
    }

    pub fn prev_field(&mut self) {
        let order = self.field_order();
        let pos = order
            .iter()
            .position(|f| *f == self.focused_field)
            .unwrap_or(0);
        self.focused_field = order[(pos + order.len() - 1) % order.len()];
        self.cursor_pos = self.active_buffer().len();
    }
}

const LOGO_LINES: &[&str] = &[
    r" ██████╗  ██████╗ ███████╗██╗   ██╗████████╗ ██████╗ ",
    r"██╔════╝ ██╔═══██╗██╔════╝██║   ██║╚══██╔══╝██╔═══██╗",
    r"██║  ███╗██║   ██║███████╗██║   ██║   ██║   ██║   ██║",
    r"██║   ██║██║   ██║╚════██║██║   ██║   ██║   ██║   ██║",
    r"╚██████╔╝╚██████╔╝███████║╚██████╔╝   ██║   ╚██████╔╝",
    r" ╚═════╝  ╚═════╝ ╚══════╝ ╚═════╝    ╚═╝    ╚═════╝ ",
];
const LOGO_TOP_BORDER: &str = "════════════════════ ゴースト ════════════════════";
const LOGO_BOTTOM_BORDER: &str = "════════════════════════════════════════════════════════";
const LOGO_HEIGHT: u16 = 9; // top border + 6 lines + bottom border + gap

pub fn render(login: &LoginState, auth_state: &AuthState, frame: &mut Frame) {
    let area = frame.area();

    let is_register = login.mode == FormMode::Register;

    let form_width = 56u16.min(area.width.saturating_sub(4));
    let form_height = if is_register { 19u16 } else { 13u16 };

    let show_logo = area.height >= form_height + LOGO_HEIGHT + 2;

    let total_height = if show_logo {
        LOGO_HEIGHT + form_height
    } else {
        form_height
    };

    let outer_area = centered_rect(form_width, total_height, area);

    let (logo_area, form_area) = if show_logo {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(LOGO_HEIGHT),
                Constraint::Length(form_height),
            ])
            .split(outer_area);
        (Some(chunks[0]), chunks[1])
    } else {
        (None, centered_rect(form_width, form_height, area))
    };

    // Render logo (no Clear — rain shows through gaps)
    if let Some(logo_rect) = logo_area {
        let mut lines = Vec::with_capacity(LOGO_HEIGHT as usize);
        lines.push(Line::from(Span::styled(
            LOGO_TOP_BORDER,
            ratatui::style::Style::default()
                .fg(theme::CYAN)
                .add_modifier(Modifier::BOLD),
        )));
        for logo_line in LOGO_LINES {
            lines.push(Line::from(Span::styled(
                *logo_line,
                ratatui::style::Style::default()
                    .fg(theme::CYAN)
                    .add_modifier(Modifier::BOLD),
            )));
        }
        lines.push(Line::from(Span::styled(
            LOGO_BOTTOM_BORDER,
            ratatui::style::Style::default()
                .fg(theme::CYAN)
                .add_modifier(Modifier::BOLD),
        )));
        // gap line (empty)
        lines.push(Line::from(""));

        frame.render_widget(
            Paragraph::new(lines).alignment(Alignment::Center),
            logo_rect,
        );
    }

    // Clear the form area so rain doesn't bleed through the panel
    frame.render_widget(Clear, form_area);

    let title = if is_register { " Register " } else { " Login " };
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            title,
            ratatui::style::Style::default()
                .fg(theme::CYAN)
                .add_modifier(Modifier::BOLD),
        )]))
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(theme::border_focused_style())
        .style(ratatui::style::Style::default().bg(theme::BLACK));

    let inner = block.inner(form_area);
    frame.render_widget(block, form_area);

    let mut constraints = vec![
        Constraint::Length(1), // [0] homeserver label
        Constraint::Length(1), // [1] homeserver input
        Constraint::Length(1), // [2] spacer
        Constraint::Length(1), // [3] username label
        Constraint::Length(1), // [4] username input
        Constraint::Length(1), // [5] spacer
        Constraint::Length(1), // [6] password label
        Constraint::Length(1), // [7] password input
    ];

    // Register mode: extra fields
    if is_register {
        constraints.push(Constraint::Length(1)); // [8] spacer
        constraints.push(Constraint::Length(1)); // [9] confirm password label
        constraints.push(Constraint::Length(1)); // [10] confirm password input
        constraints.push(Constraint::Length(1)); // [11] spacer
        constraints.push(Constraint::Length(1)); // [12] token label
        constraints.push(Constraint::Length(1)); // [13] token input
    }

    let status_idx = constraints.len();
    constraints.push(Constraint::Length(1)); // spacer before status
    constraints.push(Constraint::Length(1)); // status/error
    constraints.push(Constraint::Min(0)); // remaining

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    // Homeserver
    render_field(
        frame,
        "Homeserver:",
        &login.homeserver,
        false,
        login.focused_field == LoginField::Homeserver,
        chunks[0],
        chunks[1],
    );

    // Username
    render_field(
        frame,
        "Username:",
        &login.username,
        false,
        login.focused_field == LoginField::Username,
        chunks[3],
        chunks[4],
    );

    // Password
    render_field(
        frame,
        "Password:",
        &login.password,
        true,
        login.focused_field == LoginField::Password,
        chunks[6],
        chunks[7],
    );

    // Register-only fields
    if is_register {
        render_field(
            frame,
            "Confirm Password:",
            &login.confirm_password,
            true,
            login.focused_field == LoginField::ConfirmPassword,
            chunks[9],
            chunks[10],
        );

        render_field(
            frame,
            "Token (optional):",
            &login.registration_token,
            true,
            login.focused_field == LoginField::RegistrationToken,
            chunks[12],
            chunks[13],
        );
    }

    // Status/error
    let mode_hint = if is_register {
        "Tab: next field | Enter: register | F2: login | Ctrl+C: quit"
    } else {
        "Tab: next field | Enter: login | F2: register | Ctrl+C: quit"
    };
    let status = match auth_state {
        AuthState::LoggingIn => Span::styled("Logging in...", theme::dim_style()),
        AuthState::AutoLoggingIn => Span::styled("Auto-logging in...", theme::dim_style()),
        AuthState::Registering => Span::styled("Registering...", theme::dim_style()),
        AuthState::Error(e) => Span::styled(e.as_str(), theme::error_style()),
        _ => Span::styled(mode_hint, theme::dim_style()),
    };
    frame.render_widget(
        Paragraph::new(Line::from(status)).alignment(Alignment::Center),
        chunks[status_idx + 1],
    );

    // Cursor
    if !matches!(
        auth_state,
        AuthState::LoggingIn | AuthState::AutoLoggingIn | AuthState::Registering
    ) {
        let (cursor_chunk, offset) = match login.focused_field {
            LoginField::Homeserver => (chunks[1], login.cursor_pos),
            LoginField::Username => (chunks[4], login.cursor_pos),
            LoginField::Password => (chunks[7], login.password.len()),
            LoginField::ConfirmPassword => (chunks[10], login.confirm_password.len()),
            LoginField::RegistrationToken => (chunks[13], login.registration_token.len()),
        };
        let cursor_x = cursor_chunk.x + 2 + offset as u16;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_char_appends_at_cursor() {
        let mut state = LoginState::new();
        state.focused_field = LoginField::Homeserver;
        state.cursor_pos = state.homeserver.len();
        state.insert_char('!');
        assert_eq!(state.homeserver, "https://matrix.org!");
        assert_eq!(state.cursor_pos, "https://matrix.org!".len());
    }

    #[test]
    fn insert_char_at_beginning() {
        let mut state = LoginState::new();
        state.focused_field = LoginField::Username;
        state.cursor_pos = 0;
        state.insert_char('a');
        assert_eq!(state.username, "a");
        assert_eq!(state.cursor_pos, 1);
    }

    #[test]
    fn backspace_removes_char_before_cursor() {
        let mut state = LoginState::new();
        state.focused_field = LoginField::Username;
        state.username = "hello".to_string();
        state.cursor_pos = 5;
        state.backspace();
        assert_eq!(state.username, "hell");
        assert_eq!(state.cursor_pos, 4);
    }

    #[test]
    fn backspace_at_start_does_nothing() {
        let mut state = LoginState::new();
        state.focused_field = LoginField::Username;
        state.username = "hello".to_string();
        state.cursor_pos = 0;
        state.backspace();
        assert_eq!(state.username, "hello");
        assert_eq!(state.cursor_pos, 0);
    }

    #[test]
    fn next_field_cycles_correctly_login() {
        let mut state = LoginState::new();
        assert_eq!(state.mode, FormMode::Login);
        // Starts at Username
        assert_eq!(state.focused_field, LoginField::Username);
        state.next_field();
        assert_eq!(state.focused_field, LoginField::Password);
        state.next_field();
        assert_eq!(state.focused_field, LoginField::Homeserver);
        state.next_field();
        assert_eq!(state.focused_field, LoginField::Username);
    }

    #[test]
    fn prev_field_cycles_correctly_login() {
        let mut state = LoginState::new();
        assert_eq!(state.focused_field, LoginField::Username);
        state.prev_field();
        assert_eq!(state.focused_field, LoginField::Homeserver);
        state.prev_field();
        assert_eq!(state.focused_field, LoginField::Password);
        state.prev_field();
        assert_eq!(state.focused_field, LoginField::Username);
    }

    #[test]
    fn next_field_cycles_correctly_register() {
        let mut state = LoginState::new();
        state.toggle_mode();
        assert_eq!(state.mode, FormMode::Register);
        state.focused_field = LoginField::Password;
        state.next_field();
        assert_eq!(state.focused_field, LoginField::ConfirmPassword);
        state.next_field();
        assert_eq!(state.focused_field, LoginField::RegistrationToken);
        state.next_field();
        assert_eq!(state.focused_field, LoginField::Homeserver);
    }

    #[test]
    fn toggle_mode_snaps_field() {
        let mut state = LoginState::new();
        state.toggle_mode(); // -> Register
        state.focused_field = LoginField::ConfirmPassword;
        state.toggle_mode(); // -> Login, should snap to Password
        assert_eq!(state.focused_field, LoginField::Password);
    }

    #[test]
    fn field_switch_sets_cursor_to_end() {
        let mut state = LoginState::new();
        state.username = "user".to_string();
        state.cursor_pos = 0; // cursor at start of username
        state.next_field(); // move to password
        assert_eq!(state.cursor_pos, 0); // password is empty
        state.prev_field(); // back to username
        assert_eq!(state.cursor_pos, 4); // cursor at end of "user"
    }

    #[test]
    fn edit_homeserver_after_auto_login_populates() {
        let mut state = LoginState::new();
        // Simulate auto-login populating the homeserver field
        state.homeserver = "https://auto.server.com".to_string();
        state.username = "autouser".to_string();

        // User tabs to homeserver field to change it
        state.focused_field = LoginField::Homeserver;
        state.cursor_pos = state.homeserver.len();

        // Clear with repeated backspace
        while state.cursor_pos > 0 {
            state.backspace();
        }
        assert_eq!(state.homeserver, "");
        assert_eq!(state.cursor_pos, 0);

        // Type new domain
        for c in "https://my.server.org".chars() {
            state.insert_char(c);
        }
        assert_eq!(state.homeserver, "https://my.server.org");
        assert_eq!(state.cursor_pos, "https://my.server.org".len());
    }
}
