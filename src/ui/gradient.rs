use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::ui::theme;

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

/// Compute the gradient color for position `i` of `total` in a perimeter walk.
pub fn perimeter_color(i: usize, total: usize, start: Color, end: Color, phase: f32) -> Color {
    let angle = (i as f32 / total as f32) * std::f32::consts::TAU + phase;
    let t = (1.0 - angle.cos()) / 2.0;
    lerp_color(start, end, t)
}

/// Walk clockwise around a rectangular perimeter, calling `f(x, y, index, total)`
/// for each position. Order: top→right→bottom(reversed)→left(reversed).
pub fn walk_perimeter(area: Rect, mut f: impl FnMut(u16, u16, usize, usize)) {
    if area.width < 2 || area.height < 2 {
        return;
    }
    let total = 2 * (area.width as usize - 1) + 2 * (area.height as usize - 1);
    let x1 = area.x;
    let x2 = area.x + area.width - 1;
    let y1 = area.y;
    let y2 = area.y + area.height - 1;

    let mut i = 0;
    // Top edge (L→R)
    for x in x1..=x2 {
        f(x, y1, i, total);
        i += 1;
    }
    // Right edge (skip top corner)
    for y in (y1 + 1)..y2 {
        f(x2, y, i, total);
        i += 1;
    }
    // Bottom edge (R→L)
    for x in (x1..=x2).rev() {
        f(x, y2, i, total);
        i += 1;
    }
    // Left edge (skip corners, B→T)
    for y in ((y1 + 1)..y2).rev() {
        f(x1, y, i, total);
        i += 1;
    }
}

/// Fill a row with the standard gradient highlight (GRADIENT_HIGHLIGHT_START → END for bg,
/// BLACK → CYAN for fg, with BOLD). If `clear` is true, also sets each cell's char to space.
pub fn fill_row_highlight(
    buf: &mut Buffer,
    bounds: Rect,
    row_y: u16,
    x_start: u16,
    width: u16,
    clear: bool,
) {
    let w = width as f32;
    for x in x_start..x_start + width {
        if x >= bounds.x + bounds.width || row_y < bounds.y || row_y >= bounds.y + bounds.height {
            continue;
        }
        let t = (x - x_start) as f32 / w.max(1.0);
        let bg = lerp_color(
            theme::GRADIENT_HIGHLIGHT_START,
            theme::GRADIENT_HIGHLIGHT_END,
            t,
        );
        let fg = lerp_color(theme::BLACK, theme::CYAN, t);
        let cell = &mut buf[(x, row_y)];
        if clear {
            cell.set_char(' ');
        }
        cell.set_style(Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD));
    }
}

/// Build a gradient title Line with the standard CYAN → GRADIENT_TITLE_END gradient.
pub fn gradient_title_line(text: &str) -> Line<'static> {
    Line::from(gradient_spans(
        text,
        theme::CYAN,
        theme::GRADIENT_TITLE_END,
        true,
    ))
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
