use ratatui::style::{Color, Modifier, Style};

// Core palette (Tokyo Night "night" variant)
pub const BG: Color = Color::Rgb(12, 14, 20);
pub const SIDEBAR_BG: Color = Color::Rgb(22, 22, 30);
pub const CHAT_BG: Color = Color::Rgb(26, 27, 38);
pub const CYAN: Color = Color::Rgb(0, 255, 255);
pub const MAGENTA: Color = Color::Rgb(255, 0, 255);
pub const GREEN: Color = Color::Rgb(158, 206, 106);
pub const RED: Color = Color::Rgb(247, 118, 142);
pub const YELLOW: Color = Color::Rgb(224, 175, 104);
pub const BLUE: Color = Color::Rgb(122, 162, 247);
pub const TEXT: Color = Color::Rgb(192, 202, 245);
pub const DIM: Color = Color::Rgb(115, 122, 162);
pub const BORDER: Color = Color::Rgb(57, 75, 112);
pub const BLACK: Color = Color::Rgb(21, 22, 30);
pub const WHITE: Color = Color::Rgb(255, 255, 255);

// Mode indicator colors
pub const NORMAL_MODE_BG: Color = BLUE;
pub const INSERT_MODE_BG: Color = GREEN;
pub const COMMAND_MODE_BG: Color = MAGENTA;

// Sender name palette (rotating)
pub const SENDER_COLORS: &[Color] = &[
    CYAN,
    MAGENTA,
    Color::Rgb(158, 206, 106), // green
    Color::Rgb(255, 158, 100), // orange
    Color::Rgb(224, 175, 104), // yellow
    Color::Rgb(122, 162, 247), // blue
    Color::Rgb(247, 118, 142), // red
    Color::Rgb(42, 195, 222),  // blue1
];

pub fn sender_color(sender: &str) -> Color {
    let hash: usize = sender.bytes().map(|b| b as usize).sum();
    SENDER_COLORS[hash % SENDER_COLORS.len()]
}

// Semantic colors
pub const HIGHLIGHT_BG: Color = Color::Rgb(41, 46, 66);
pub const MUTED: Color = Color::Rgb(59, 66, 97);
pub const BAR_EMPTY: Color = Color::Rgb(65, 72, 104);
pub const METER_EMPTY: Color = Color::Rgb(41, 46, 66);
pub const MESSAGE_SELECT_BG: Color = Color::Rgb(40, 52, 87);
pub const REPLY_INDICATOR: Color = Color::Rgb(122, 162, 247);
pub const REACTION_BG: Color = Color::Rgb(41, 46, 66);
pub const REACTION_OWN_BG: Color = Color::Rgb(57, 75, 112);
pub const EDIT_INDICATOR: Color = Color::Rgb(115, 218, 202);

// Gradient endpoints (neon — kept as-is)
pub const GRADIENT_BORDER_START: Color = Color::Rgb(0, 255, 255); // CYAN
pub const GRADIENT_BORDER_END: Color = Color::Rgb(255, 0, 255); // MAGENTA
pub const GRADIENT_TITLE_END: Color = Color::Rgb(100, 255, 255); // lighter cyan
pub const GRADIENT_HIGHLIGHT_START: Color = Color::Rgb(0, 255, 255); // bright left edge
pub const GRADIENT_HIGHLIGHT_END: Color = Color::Rgb(0, 80, 120); // deep teal right edge
pub const GRADIENT_DATE_BRIGHT: Color = Color::Rgb(115, 122, 162); // center of date sep (TN dark5)
pub const GRADIENT_DATE_DIM: Color = Color::Rgb(59, 66, 97); // edge of date sep (TN fg_gutter)
pub const STATUS_BAR_BG: Color = Color::Rgb(22, 22, 30);
pub const UNREAD_BADGE_BG: Color = Color::Rgb(57, 75, 112);
pub const TIMESTAMP_BRIGHT: Color = Color::Rgb(115, 122, 162);
pub const INPUT_BORDER_GREEN_DIM: Color = Color::Rgb(0, 160, 80); // darker green
pub const INPUT_BORDER_MAGENTA_DIM: Color = Color::Rgb(160, 0, 160); // darker magenta

// Rich text / formatted message colors
pub const CODE_INLINE_FG: Color = Color::Rgb(187, 154, 247);
pub const CODE_INLINE_BG: Color = Color::Rgb(41, 46, 66);
pub const CODE_BLOCK_BG: Color = Color::Rgb(22, 22, 30);
pub const LINK_FG: Color = Color::Rgb(122, 162, 247);
pub const BLOCKQUOTE_FG: Color = Color::Rgb(115, 122, 162);

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

pub fn dim_italic_style() -> Style {
    Style::default().fg(DIM).add_modifier(Modifier::ITALIC)
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
