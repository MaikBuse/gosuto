pub mod emp_pulse;
pub mod glitch;
pub mod matrix_rain;
pub mod text_reveal;

pub use text_reveal::TextReveal;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Color;

use super::theme;
use emp_pulse::EmpPulse;
use glitch::GlitchEffect;
use matrix_rain::MatrixRain;

/// Minimal XorShift64 PRNG — no external dependency needed
pub(crate) struct Xorshift64(u64);

impl Xorshift64 {
    pub fn new(seed: u64) -> Self {
        Self(if seed == 0 {
            0xDEAD_BEEF_CAFE_BABE
        } else {
            seed
        })
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
    pub emp_pulse: EmpPulse,
    pub members_emp_pulse: EmpPulse,
}

impl EffectsState {
    pub fn new(rain_enabled: bool, glitch_enabled: bool) -> Self {
        Self {
            enabled: rain_enabled,
            matrix_rain: MatrixRain::new(),
            glitch_enabled,
            glitch: GlitchEffect::new(),
            emp_pulse: EmpPulse::new(),
            members_emp_pulse: EmpPulse::new(),
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

    pub fn tick_emp(&mut self, dt_ms: u64, area: Rect, focused: bool) {
        self.emp_pulse.tick(dt_ms, area, focused);
    }

    pub fn tick_members_emp(&mut self, dt_ms: u64, area: Rect, focused: bool) {
        self.members_emp_pulse.tick(dt_ms, area, focused);
    }

    pub fn render_emp_buffer(&self, area: Rect, scroll_offset: usize) -> Option<Buffer> {
        if !self.enabled {
            return None;
        }
        let mut buf = Buffer::empty(area);
        self.emp_pulse.render(&mut buf, area, scroll_offset);
        Some(buf)
    }

    pub fn render_members_emp_buffer(&self, area: Rect, scroll_offset: usize) -> Option<Buffer> {
        if !self.enabled {
            return None;
        }
        let mut buf = Buffer::empty(area);
        self.members_emp_pulse.render(&mut buf, area, scroll_offset);
        Some(buf)
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
/// Cells with `skip=true` (used by ratatui-image for iTerm2/Sixel/Kitty protocols) are
/// never overwritten, so the terminal's image overlay is preserved.
pub fn composite(frame_buf: &mut Buffer, effect_buf: &Buffer, area: Rect) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = &frame_buf[(x, y)];
            if !cell.skip && is_transparent_cell(cell) {
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- Xorshift64 ---

    #[test]
    fn xorshift_zero_seed_uses_fallback() {
        let mut rng = Xorshift64::new(0);
        // Should not be stuck at 0 — fallback seed is used
        let val = rng.next();
        assert_ne!(val, 0);
    }

    #[test]
    fn xorshift_deterministic_sequence() {
        let mut rng1 = Xorshift64::new(42);
        let mut rng2 = Xorshift64::new(42);
        for _ in 0..100 {
            assert_eq!(rng1.next(), rng2.next());
        }
    }

    #[test]
    fn xorshift_different_seeds_differ() {
        let mut rng1 = Xorshift64::new(1);
        let mut rng2 = Xorshift64::new(2);
        // Very unlikely to produce the same first value
        assert_ne!(rng1.next(), rng2.next());
    }

    #[test]
    fn xorshift_next_f32_in_range() {
        let mut rng = Xorshift64::new(123);
        for _ in 0..1000 {
            let val = rng.next_f32();
            assert!((0.0..=1.0).contains(&val), "next_f32 out of range: {val}");
        }
    }

    #[test]
    fn xorshift_next_range_bounds() {
        let mut rng = Xorshift64::new(456);
        for _ in 0..1000 {
            let val = rng.next_range(10.0, 20.0);
            assert!(
                (10.0..=20.0).contains(&val),
                "next_range out of bounds: {val}"
            );
        }
    }

    #[test]
    fn xorshift_next_u32_range_min_eq_max() {
        let mut rng = Xorshift64::new(789);
        let val = rng.next_u32_range(5, 5);
        assert_eq!(val, 5);
    }

    #[test]
    fn xorshift_next_u32_range_values_in_range() {
        let mut rng = Xorshift64::new(101);
        for _ in 0..1000 {
            let val = rng.next_u32_range(3, 10);
            assert!(
                (3..10).contains(&val),
                "next_u32_range out of bounds: {val}"
            );
        }
    }

    #[test]
    fn xorshift_next_u32_range_min_gt_max() {
        let mut rng = Xorshift64::new(202);
        // When min >= max, should return min
        let val = rng.next_u32_range(10, 5);
        assert_eq!(val, 10);
    }

    // --- EffectsState ---

    #[test]
    fn effects_initial_state() {
        let state = EffectsState::new(true, false);
        assert!(state.enabled);
        assert!(!state.glitch_enabled);
    }

    #[test]
    fn effects_toggle_flips() {
        let mut state = EffectsState::new(false, false);
        assert!(!state.enabled);
        state.toggle();
        assert!(state.enabled);
        state.toggle();
        assert!(!state.enabled);
    }

    #[test]
    fn effects_toggle_glitch_flips() {
        let mut state = EffectsState::new(false, false);
        assert!(!state.glitch_enabled);
        state.toggle_glitch();
        assert!(state.glitch_enabled);
        state.toggle_glitch();
        assert!(!state.glitch_enabled);
    }

    #[test]
    fn effects_render_to_buffer_disabled() {
        let state = EffectsState::new(false, false);
        let area = Rect::new(0, 0, 10, 10);
        assert!(state.render_to_buffer(area).is_none());
    }

    #[test]
    fn effects_render_emp_buffer_disabled() {
        let state = EffectsState::new(false, false);
        let area = Rect::new(0, 0, 10, 10);
        assert!(state.render_emp_buffer(area, 0).is_none());
    }

    #[test]
    fn effects_render_members_emp_buffer_disabled() {
        let state = EffectsState::new(false, false);
        let area = Rect::new(0, 0, 10, 10);
        assert!(state.render_members_emp_buffer(area, 0).is_none());
    }
}
