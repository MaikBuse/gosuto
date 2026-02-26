use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use super::{EffectLayer, Xorshift64};

/// Half-width Katakana range (U+FF66–FF9D): single-cell-wide in terminals
const KATAKANA_START: u32 = 0xFF66;
const KATAKANA_END: u32 = 0xFF9D;
const KATAKANA_COUNT: u32 = KATAKANA_END - KATAKANA_START + 1;

trait XorshiftCharExt {
    fn next_char(&mut self) -> char;
}

impl XorshiftCharExt for Xorshift64 {
    fn next_char(&mut self) -> char {
        let idx = (self.next() % KATAKANA_COUNT as u64) as u32;
        char::from_u32(KATAKANA_START + idx).unwrap_or('ア')
    }
}

struct RainColumn {
    head_y: f32,
    speed: f32,
    trail_len: u16,
    active: bool,
    delay_ms: f32,
    chars: Vec<char>,
}

pub struct MatrixRain {
    columns: Vec<RainColumn>,
    rng: Xorshift64,
    last_area: Rect,
    char_cycle_accum: f32,
}

impl MatrixRain {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            rng: Xorshift64::new(0xB00B_FACE_1337_C0DE),
            last_area: Rect::default(),
            char_cycle_accum: 0.0,
        }
    }

    fn init_columns(&mut self, area: Rect) {
        self.last_area = area;
        self.columns.clear();
        self.columns.reserve(area.width as usize);

        for _ in 0..area.width {
            let active = self.rng.next_f32() < 0.70;
            let speed = self.rng.next_range(4.0, 16.0);
            let trail_len = self.rng.next_range(4.0, (area.height as f32 * 0.8).max(6.0)) as u16;
            let delay = if active {
                self.rng.next_range(0.0, 2000.0)
            } else {
                self.rng.next_range(500.0, 4000.0)
            };

            let mut chars = Vec::with_capacity(trail_len as usize + 1);
            for _ in 0..=trail_len {
                chars.push(self.rng.next_char());
            }

            self.columns.push(RainColumn {
                head_y: -(self.rng.next_range(0.0, area.height as f32)),
                speed,
                trail_len,
                active,
                delay_ms: delay,
                chars,
            });
        }
    }

    fn reset_column(&mut self, idx: usize, area: Rect) {
        let col = &mut self.columns[idx];
        col.speed = self.rng.next_range(4.0, 16.0);
        col.trail_len =
            self.rng.next_range(4.0, (area.height as f32 * 0.8).max(6.0)) as u16;
        col.head_y = -(self.rng.next_range(0.0, 4.0));
        col.delay_ms = self.rng.next_range(200.0, 3000.0);
        col.active = true;

        col.chars.clear();
        for _ in 0..=col.trail_len {
            col.chars.push(self.rng.next_char());
        }
    }
}

impl EffectLayer for MatrixRain {
    fn tick(&mut self, dt_ms: u64, area: Rect) {
        // Reinitialize on first tick or terminal resize
        if area != self.last_area || self.columns.is_empty() {
            self.init_columns(area);
            return;
        }

        let dt_sec = dt_ms as f32 / 1000.0;
        let height = area.height as f32;

        // Cycle random characters periodically (~150ms)
        self.char_cycle_accum += dt_ms as f32;
        let cycle_chars = self.char_cycle_accum >= 150.0;
        if cycle_chars {
            self.char_cycle_accum -= 150.0;
        }

        let col_count = self.columns.len();
        for i in 0..col_count {
            let col = &mut self.columns[i];

            // Handle delay before column starts falling
            if col.delay_ms > 0.0 {
                col.delay_ms -= dt_ms as f32;
                continue;
            }

            if !col.active {
                col.active = true;
            }

            col.head_y += col.speed * dt_sec;

            // Cycle one random character in the trail
            if cycle_chars && !col.chars.is_empty() {
                let char_idx = (self.rng.next() % col.chars.len() as u64) as usize;
                col.chars[char_idx] = self.rng.next_char();
            }

            // Reset column when entire trail has fallen off screen
            let tail_y = col.head_y - col.trail_len as f32;
            if tail_y > height {
                self.reset_column(i, area);
            }
        }
    }

    fn render(&self, buf: &mut Buffer) {
        let area = self.last_area;
        if area.width == 0 || area.height == 0 {
            return;
        }

        for (col_x, col) in self.columns.iter().enumerate() {
            if !col.active || col.delay_ms > 0.0 {
                continue;
            }

            let x = area.x + col_x as u16;
            if x >= area.x + area.width {
                break;
            }

            let head_row = col.head_y as i32;

            for trail_i in 0..=col.trail_len as i32 {
                let row = head_row - trail_i;
                if row < area.y as i32 || row >= (area.y + area.height) as i32 {
                    continue;
                }

                let char_idx = (trail_i as usize) % col.chars.len().max(1);
                let ch = col.chars[char_idx];

                let color = if trail_i == 0 {
                    // Head: bright white-green
                    Color::Rgb(200, 255, 200)
                } else {
                    // Trail: quadratic falloff
                    let t = trail_i as f32 / col.trail_len as f32;
                    let t2 = t * t;
                    let r = 0;
                    let g = (180.0 - 165.0 * t2) as u8;
                    let b = (60.0 - 55.0 * t2) as u8;
                    Color::Rgb(r, g, b)
                };

                let cell = &mut buf[(x, row as u16)];
                cell.set_char(ch);
                cell.set_style(Style::default().fg(color).bg(Color::Reset));
            }
        }
    }

    fn is_active(&self) -> bool {
        !self.columns.is_empty()
    }
}
