pub mod matrix_rain;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use super::theme;
use matrix_rain::MatrixRain;

#[allow(dead_code)]
pub trait EffectLayer {
    fn tick(&mut self, dt_ms: u64, area: Rect);
    fn render(&self, buf: &mut Buffer);
    fn is_active(&self) -> bool;
}

pub struct EffectsState {
    pub enabled: bool,
    matrix_rain: MatrixRain,
}

impl EffectsState {
    pub fn new() -> Self {
        Self {
            enabled: false,
            matrix_rain: MatrixRain::new(),
        }
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn tick(&mut self, dt_ms: u64, area: Rect) {
        if !self.enabled {
            return;
        }
        self.matrix_rain.tick(dt_ms, area);
    }

    pub fn render_to_buffer(&self, area: Rect) -> Option<Buffer> {
        if !self.enabled {
            return None;
        }
        let mut buf = Buffer::empty(area);
        self.matrix_rain.render(&mut buf);
        Some(buf)
    }
}

/// Composite effect buffer behind the UI: replace "transparent" UI cells with effect cells.
pub fn composite(frame_buf: &mut Buffer, effect_buf: &Buffer, area: Rect) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if is_transparent_cell(&frame_buf[(x, y)]) {
                frame_buf[(x, y)] = effect_buf[(x, y)].clone();
            }
        }
    }
}

/// A UI cell is "transparent" if it has the default background color (or Reset)
/// and contains only whitespace — meaning there's nothing meaningful drawn there.
fn is_transparent_cell(cell: &ratatui::buffer::Cell) -> bool {
    let bg = cell.bg;
    let bg_is_default = bg == theme::BG || bg == Color::Reset;
    let symbol_is_empty = cell.symbol().trim().is_empty();
    bg_is_default && symbol_is_empty
}
