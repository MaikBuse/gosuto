use ratatui::style::{Color, Modifier, Style};

// Core palette
pub const BG: Color = Color::Rgb(10, 10, 15);
pub const CYAN: Color = Color::Rgb(0, 255, 255);
pub const MAGENTA: Color = Color::Rgb(255, 0, 255);
pub const GREEN: Color = Color::Rgb(0, 255, 128);
pub const RED: Color = Color::Rgb(255, 80, 60);
pub const YELLOW: Color = Color::Rgb(255, 200, 50);
pub const TEXT: Color = Color::Rgb(220, 220, 230);
pub const DIM: Color = Color::Rgb(120, 120, 140);
pub const BORDER: Color = Color::Rgb(40, 50, 60);
pub const BLACK: Color = Color::Rgb(0, 0, 0);
pub const WHITE: Color = Color::Rgb(255, 255, 255);

// Mode indicator colors
pub const NORMAL_MODE_BG: Color = CYAN;
pub const INSERT_MODE_BG: Color = GREEN;
pub const COMMAND_MODE_BG: Color = MAGENTA;

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

// Semantic colors
pub const HIGHLIGHT_BG: Color = Color::Rgb(20, 20, 40);
pub const MUTED: Color = Color::Rgb(60, 60, 80);
pub const BAR_EMPTY: Color = Color::Rgb(40, 40, 50);
pub const METER_EMPTY: Color = Color::Rgb(30, 30, 40);
pub const MESSAGE_SELECT_BG: Color = Color::Rgb(25, 30, 50);
pub const REPLY_INDICATOR: Color = Color::Rgb(100, 180, 255);
pub const REACTION_BG: Color = Color::Rgb(30, 35, 55);
pub const REACTION_OWN_BG: Color = Color::Rgb(25, 50, 70);
pub const EDIT_INDICATOR: Color = Color::Rgb(100, 200, 150);

// Gradient endpoints
pub const GRADIENT_BORDER_START: Color = Color::Rgb(0, 255, 255); // CYAN
pub const GRADIENT_BORDER_END: Color = Color::Rgb(255, 0, 255); // MAGENTA
pub const GRADIENT_TITLE_END: Color = Color::Rgb(100, 255, 255); // lighter cyan
pub const GRADIENT_HIGHLIGHT_START: Color = Color::Rgb(0, 255, 255); // bright left edge
pub const GRADIENT_HIGHLIGHT_END: Color = Color::Rgb(0, 80, 120); // deep teal right edge
pub const GRADIENT_DATE_BRIGHT: Color = Color::Rgb(100, 100, 120); // center of date sep
pub const GRADIENT_DATE_DIM: Color = Color::Rgb(30, 30, 45); // edge of date sep
pub const STATUS_BAR_BG: Color = Color::Rgb(15, 15, 22); // slightly lighter than BG
pub const UNREAD_BADGE_BG: Color = Color::Rgb(0, 40, 50); // subtle cyan tint
pub const TIMESTAMP_BRIGHT: Color = Color::Rgb(140, 140, 160); // slightly brighter than DIM
pub const INPUT_BORDER_GREEN_DIM: Color = Color::Rgb(0, 160, 80); // darker green
pub const INPUT_BORDER_MAGENTA_DIM: Color = Color::Rgb(160, 0, 160); // darker magenta
pub const PULSE_BASE: Color = Color::Rgb(20, 255, 255); // room list pulse base

// Rich text / formatted message colors
pub const CODE_INLINE_FG: Color = Color::Rgb(220, 180, 255); // light purple for inline code
pub const CODE_INLINE_BG: Color = Color::Rgb(30, 25, 45); // subtle purple tint background
pub const CODE_BLOCK_BG: Color = Color::Rgb(20, 20, 30); // dark background for code blocks
pub const LINK_FG: Color = Color::Rgb(100, 180, 255); // blue for hyperlinks
pub const BLOCKQUOTE_FG: Color = Color::Rgb(140, 140, 160); // muted for block quotes

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

// Form field styles
pub fn field_marker_style(selected: bool) -> Style {
    let color = if selected { CYAN } else { DIM };
    Style::default().fg(color).bg(BG)
}

pub fn field_label_style(selected: bool) -> Style {
    let color = if selected { CYAN } else { TEXT };
    let modifier = if selected {
        Modifier::BOLD
    } else {
        Modifier::empty()
    };
    Style::default().fg(color).bg(BG).add_modifier(modifier)
}

pub fn field_value_style(selected: bool) -> Style {
    let color = if selected { TEXT } else { DIM };
    Style::default().fg(color).bg(BG)
}

pub fn field_arrow_style(selected: bool) -> Style {
    let color = if selected { CYAN } else { DIM };
    Style::default().fg(color).bg(BG)
}

pub fn edit_text_style() -> Style {
    Style::default().fg(GREEN).bg(BG)
}

pub fn edit_cursor_style() -> Style {
    Style::default().fg(GREEN).bg(BG)
}

pub fn highlight_focused_style() -> Style {
    Style::default()
        .fg(BLACK)
        .bg(CYAN)
        .add_modifier(Modifier::BOLD)
}

pub fn highlight_unfocused_style() -> Style {
    Style::default().fg(CYAN).bg(HIGHLIGHT_BG)
}

pub fn message_selected_style() -> Style {
    Style::default().bg(MESSAGE_SELECT_BG)
}

pub fn reply_indicator_style() -> Style {
    Style::default().fg(REPLY_INDICATOR)
}

pub fn reaction_badge_style() -> Style {
    Style::default().fg(TEXT).bg(REACTION_BG)
}

pub fn reaction_own_badge_style() -> Style {
    Style::default().fg(CYAN).bg(REACTION_OWN_BG)
}

pub fn edit_indicator_style() -> Style {
    Style::default()
        .fg(EDIT_INDICATOR)
        .add_modifier(Modifier::BOLD)
}

pub fn code_inline_style() -> Style {
    Style::default().fg(CODE_INLINE_FG).bg(CODE_INLINE_BG)
}

pub fn code_block_style() -> Style {
    Style::default().fg(TEXT).bg(CODE_BLOCK_BG)
}

pub fn link_style() -> Style {
    Style::default()
        .fg(LINK_FG)
        .add_modifier(Modifier::UNDERLINED)
}

pub fn blockquote_style() -> Style {
    Style::default().fg(BLOCKQUOTE_FG)
}

pub fn loading_style() -> Style {
    Style::default().fg(CYAN).bg(BG)
}

pub fn saving_style() -> Style {
    Style::default()
        .fg(GREEN)
        .bg(BG)
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
