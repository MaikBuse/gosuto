use ratatui::style::{Color, Modifier, Style};

// Core palette
pub const BG: Color = Color::Rgb(10, 10, 15);
pub const CYAN: Color = Color::Rgb(0, 255, 255);
pub const MAGENTA: Color = Color::Rgb(255, 0, 255);
pub const GREEN: Color = Color::Rgb(0, 255, 128);
pub const RED: Color = Color::Rgb(255, 80, 60);
pub const TEXT: Color = Color::Rgb(220, 220, 230);
pub const DIM: Color = Color::Rgb(120, 120, 140);
pub const BORDER: Color = Color::Rgb(40, 50, 60);
pub const BLACK: Color = Color::Rgb(0, 0, 0);

// Sender name palette (rotating)
pub const SENDER_COLORS: &[Color] = &[
    CYAN,
    MAGENTA,
    GREEN,
    Color::Rgb(255, 165, 0),   // orange
    Color::Rgb(255, 255, 0),   // yellow
    Color::Rgb(128, 128, 255), // periwinkle
    Color::Rgb(255, 100, 200), // pink
    Color::Rgb(0, 200, 255),   // sky blue
];

pub fn sender_color(sender: &str) -> Color {
    let hash: usize = sender.bytes().map(|b| b as usize).sum();
    SENDER_COLORS[hash % SENDER_COLORS.len()]
}

// Composite styles
pub fn border_style() -> Style {
    Style::default().fg(BORDER)
}

pub fn border_focused_style() -> Style {
    Style::default().fg(CYAN)
}

pub fn title_style() -> Style {
    Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
}

pub fn text_style() -> Style {
    Style::default().fg(TEXT)
}

pub fn dim_style() -> Style {
    Style::default().fg(DIM)
}

pub fn error_style() -> Style {
    Style::default().fg(RED)
}

pub fn normal_mode_style() -> Style {
    Style::default()
        .fg(BLACK)
        .bg(CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn insert_mode_style() -> Style {
    Style::default()
        .fg(BLACK)
        .bg(GREEN)
        .add_modifier(Modifier::BOLD)
}

pub fn command_mode_style() -> Style {
    Style::default()
        .fg(BLACK)
        .bg(MAGENTA)
        .add_modifier(Modifier::BOLD)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sender_color_deterministic() {
        let c1 = sender_color("@alice:matrix.org");
        let c2 = sender_color("@alice:matrix.org");
        assert_eq!(c1, c2);
    }

    #[test]
    fn sender_color_different_inputs() {
        let c1 = sender_color("@alice:matrix.org");
        let c2 = sender_color("@bob:matrix.org");
        // Different inputs should produce results (may or may not be different
        // colors due to hash collisions, but the function should not panic)
        let _ = (c1, c2);
    }

    #[test]
    fn sender_color_empty_string() {
        let c = sender_color("");
        // 0 % 8 == 0, so should return SENDER_COLORS[0] which is CYAN
        assert_eq!(c, SENDER_COLORS[0]);
    }

    #[test]
    fn sender_color_returns_from_palette() {
        let c = sender_color("test_user");
        assert!(SENDER_COLORS.contains(&c));
    }
}
