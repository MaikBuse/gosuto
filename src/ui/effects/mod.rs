pub mod glitch;
pub mod matrix_rain;
pub mod text_reveal;

pub use text_reveal::TextReveal;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use super::theme;
use glitch::GlitchEffect;
use matrix_rain::MatrixRain;

/// Minimal XorShift64 PRNG — no external dependency needed
pub(crate) struct Xorshift64(u64);

impl Xorshift64 {
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 { 0xDEAD_BEEF_CAFE_BABE } else { seed })
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn next_f32(&mut self) -> f32 {
        (self.next() & 0xFFFF) as f32 / 65535.0
    }

    pub fn next_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }

    pub fn next_u32_range(&mut self, min: u32, max: u32) -> u32 {
        if min >= max {
            return min;
        }
        let range = max - min;
        min + (self.next() % range as u64) as u32
    }
}

#[allow(dead_code)]
pub trait EffectLayer {
    fn tick(&mut self, dt_ms: u64, area: Rect);
    fn render(&self, buf: &mut Buffer);
    fn is_active(&self) -> bool;
}

pub struct EffectsState {
    pub enabled: bool,
    matrix_rain: MatrixRain,
    pub glitch_enabled: bool,
    glitch: GlitchEffect,
}

impl EffectsState {
    pub fn new(rain_enabled: bool, glitch_enabled: bool) -> Self {
        Self {
            enabled: rain_enabled,
            matrix_rain: MatrixRain::new(),
            glitch_enabled,
            glitch: GlitchEffect::new(),
        }
    }

    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
    }

    pub fn toggle_glitch(&mut self) {
        self.glitch_enabled = !self.glitch_enabled;
    }

    pub fn tick(&mut self, dt_ms: u64, area: Rect) {
        if self.enabled {
            self.matrix_rain.tick(dt_ms, area);
        }
        if self.glitch_enabled {
            self.glitch.tick(dt_ms, area.height);
        }
    }

    pub fn render_to_buffer(&self, area: Rect) -> Option<Buffer> {
        if !self.enabled {
            return None;
        }
        let mut buf = Buffer::empty(area);
        self.matrix_rain.render(&mut buf);
        Some(buf)
    }

    pub fn post_process_glitch(&self, buf: &mut Buffer, areas: &[Rect]) {
        if self.glitch_enabled {
            self.glitch.post_process(buf, areas);
        }
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

/// A UI cell is "transparent" if it has default background, default foreground,
/// and contains only whitespace — meaning there's nothing meaningful drawn there.
/// Cells styled by widgets (titles, text spans) have explicit fg colors, so they
/// won't be treated as transparent even when their symbol is a space.
fn is_transparent_cell(cell: &ratatui::buffer::Cell) -> bool {
    let bg = cell.bg;
    let bg_is_default = bg == theme::BG || bg == Color::Reset;
    let symbol_is_empty = cell.symbol().trim().is_empty();
    let fg_is_default = cell.fg == Color::Reset;
    bg_is_default && symbol_is_empty && fg_is_default
}
