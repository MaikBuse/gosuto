use ratatui::{
    Frame,
    layout::Rect,
    text::Span,
};

use crate::app::App;
use crate::ui::theme;

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let room_id = match app.messages.current_room_id.as_ref() {
        Some(id) => id,
        None => return,
    };
    let names = match app.typing_users.get(room_id) {
        Some(names) if !names.is_empty() => names,
        _ => return,
    };

    let text = format_typing(names);
    let span = Span::styled(format!(" {text}"), theme::dim_style());
    frame.render_widget(span, area);
}

pub fn format_typing(names: &[String]) -> String {
    match names.len() {
        0 => String::new(),
        1 => format!("{} is typing...", names[0]),
        2 => format!("{} and {} are typing...", names[0], names[1]),
        n => format!(
            "{}, {} and {} others are typing...",
            names[0],
            names[1],
            n - 2
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_empty() {
        assert_eq!(format_typing(&[]), "");
    }

    #[test]
    fn format_one() {
        let names = vec!["Alice".to_string()];
        assert_eq!(format_typing(&names), "Alice is typing...");
    }

    #[test]
    fn format_two() {
        let names = vec!["Alice".to_string(), "Bob".to_string()];
        assert_eq!(format_typing(&names), "Alice and Bob are typing...");
    }

    #[test]
    fn format_three() {
        let names = vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
        ];
        assert_eq!(
            format_typing(&names),
            "Alice, Bob and 1 others are typing..."
        );
    }

    #[test]
    fn format_four() {
        let names = vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Charlie".to_string(),
            "Dave".to_string(),
        ];
        assert_eq!(
            format_typing(&names),
            "Alice, Bob and 2 others are typing..."
        );
    }
}
