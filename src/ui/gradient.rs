use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Span;

/// Linearly interpolate between two RGB colors. Clamps `t` to [0.0, 1.0].
/// Falls back to `a` if either color is not RGB.
pub fn lerp_color(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    match (a, b) {
        (Color::Rgb(r1, g1, b1), Color::Rgb(r2, g2, b2)) => {
            let r = (r1 as f32 + (r2 as f32 - r1 as f32) * t).round() as u8;
            let g = (g1 as f32 + (g2 as f32 - g1 as f32) * t).round() as u8;
            let b = (b1 as f32 + (b2 as f32 - b1 as f32) * t).round() as u8;
            Color::Rgb(r, g, b)
        }
        _ => a,
    }
}

/// Scale an RGB color's brightness by `factor`. Values > 1.0 brighten, < 1.0 darken.
/// Falls back to the original color if not RGB.
pub fn scale_color(color: Color, factor: f32) -> Color {
    match color {
        Color::Rgb(r, g, b) => {
            let r = ((r as f32) * factor).round().clamp(0.0, 255.0) as u8;
            let g = ((g as f32) * factor).round().clamp(0.0, 255.0) as u8;
            let b = ((b as f32) * factor).round().clamp(0.0, 255.0) as u8;
            Color::Rgb(r, g, b)
        }
        _ => color,
    }
}

/// Build per-character gradient spans from `start` to `end` color.
pub fn gradient_spans(text: &str, start: Color, end: Color, bold: bool) -> Vec<Span<'static>> {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    if len == 0 {
        return vec![];
    }
    let divisor = if len > 1 { (len - 1) as f32 } else { 1.0 };
    chars
        .into_iter()
        .enumerate()
        .map(|(i, ch)| {
            let t = i as f32 / divisor;
            let color = lerp_color(start, end, t);
            let mut style = Style::default().fg(color);
            if bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            Span::styled(ch.to_string(), style)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_at_zero() {
        let a = Color::Rgb(0, 0, 0);
        let b = Color::Rgb(255, 255, 255);
        assert_eq!(lerp_color(a, b, 0.0), a);
    }

    #[test]
    fn lerp_at_one() {
        let a = Color::Rgb(0, 0, 0);
        let b = Color::Rgb(255, 255, 255);
        assert_eq!(lerp_color(a, b, 1.0), b);
    }

    #[test]
    fn lerp_at_half() {
        let a = Color::Rgb(0, 0, 0);
        let b = Color::Rgb(200, 100, 50);
        assert_eq!(lerp_color(a, b, 0.5), Color::Rgb(100, 50, 25));
    }

    #[test]
    fn lerp_clamps_t() {
        let a = Color::Rgb(10, 20, 30);
        let b = Color::Rgb(100, 200, 250);
        assert_eq!(lerp_color(a, b, -1.0), a);
        assert_eq!(lerp_color(a, b, 2.0), b);
    }

    #[test]
    fn lerp_non_rgb_fallback() {
        let a = Color::Red;
        let b = Color::Rgb(255, 0, 0);
        assert_eq!(lerp_color(a, b, 0.5), Color::Red);
    }

    #[test]
    fn scale_color_half() {
        let c = Color::Rgb(200, 100, 50);
        assert_eq!(scale_color(c, 0.5), Color::Rgb(100, 50, 25));
    }

    #[test]
    fn scale_color_clamps() {
        let c = Color::Rgb(200, 200, 200);
        assert_eq!(scale_color(c, 2.0), Color::Rgb(255, 255, 255));
    }

    #[test]
    fn scale_color_non_rgb_fallback() {
        assert_eq!(scale_color(Color::Blue, 0.5), Color::Blue);
    }

    #[test]
    fn gradient_spans_empty() {
        assert!(
            gradient_spans("", Color::Rgb(0, 0, 0), Color::Rgb(255, 255, 255), false).is_empty()
        );
    }

    #[test]
    fn gradient_spans_single_char() {
        let spans = gradient_spans("X", Color::Rgb(0, 0, 0), Color::Rgb(255, 255, 255), true);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "X");
    }

    #[test]
    fn gradient_spans_length() {
        let spans = gradient_spans(
            "hello",
            Color::Rgb(0, 0, 0),
            Color::Rgb(255, 255, 255),
            false,
        );
        assert_eq!(spans.len(), 5);
    }

    #[test]
    fn gradient_spans_bold_modifier() {
        let spans = gradient_spans("AB", Color::Rgb(0, 0, 0), Color::Rgb(255, 255, 255), true);
        for span in &spans {
            assert!(span.style.add_modifier.contains(Modifier::BOLD));
        }
    }
}
