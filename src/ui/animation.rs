pub struct AnimationClock {
    /// Phase in radians [0, TAU)
    pub phase: f32,
    /// Cursor blink phase in radians [0, TAU), ticked at ~1200ms full cycle.
    cursor_phase: f32,
}

impl AnimationClock {
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            cursor_phase: 0.0,
        }
    }

    pub fn tick(&mut self, dt_ms: u64) {
        self.phase += (dt_ms as f32 / 2000.0) * std::f32::consts::TAU;
        if self.phase > std::f32::consts::TAU {
            self.phase -= std::f32::consts::TAU;
        }
        self.cursor_phase += (dt_ms as f32 / 1200.0) * std::f32::consts::TAU;
        if self.cursor_phase > std::f32::consts::TAU {
            self.cursor_phase -= std::f32::consts::TAU;
        }
    }

    /// Whether a blinking cursor should be visible right now (~600ms on/off).
    pub fn cursor_visible(&self) -> bool {
        self.cursor_phase.sin() >= 0.0
    }

    /// Reset blink so the cursor is immediately visible (call on input).
    pub fn reset_cursor(&mut self) {
        self.cursor_phase = 0.0;
    }
}
