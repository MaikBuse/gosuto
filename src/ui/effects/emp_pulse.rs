use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use super::Xorshift64;

// ── Character sets ──────────────────────────────────────────────────────────

const WAVE_CHARS: &[char] = &['━', '═', '─', '╌', '┄'];
const SPARK_CHARS: &[char] = &['⡟', '⣏', '⠿', '⣿', '⡷', '⣇', '⢿', '⣾', '⡿', '⠟'];
const FIELD_CHARS: &[char] = &['░', '╎', '┊', '│'];

// ── Sub-effect structs ──────────────────────────────────────────────────────

struct Shockwave {
    epicenter_row: u16,
    elapsed_ms: f32,
    duration_ms: f32,
}

struct Spark {
    x: u16,
    y: u16,
    char_idx: usize,
    remaining_ms: f32,
    intensity: f32,
}

struct FieldLine {
    x: u16,
    y: f32,
    length: u16,
    speed: f32,
    brightness: f32,
}

struct MicroPulse {
    row: u16,
    elapsed_ms: f32,
    duration_ms: f32,
}

// ── Main effect ─────────────────────────────────────────────────────────────

pub struct EmpPulse {
    shockwaves: Vec<Shockwave>,
    sparks: Vec<Spark>,
    field_lines: Vec<FieldLine>,
    micro_pulses: Vec<MicroPulse>,
    rng: Xorshift64,
    last_area: Rect,
    ambient_timer: f32,
}

impl EmpPulse {
    pub fn new() -> Self {
        Self {
            shockwaves: Vec::new(),
            sparks: Vec::new(),
            field_lines: Vec::new(),
            micro_pulses: Vec::new(),
            rng: Xorshift64::new(0xE1EC_7A0F_0015_E000),
            last_area: Rect::default(),
            ambient_timer: 0.0,
        }
    }

    /// Advance all animations by `dt_ms` milliseconds.
    pub fn tick(&mut self, dt_ms: u64, area: Rect, _focused: bool) {
        let dt = dt_ms as f32;

        // Re-init field lines on resize
        if area != self.last_area || self.field_lines.is_empty() {
            self.last_area = area;
            self.init_field_lines(area);
        }

        // ── Shockwaves ──────────────────────────────────────────────────
        for sw in &mut self.shockwaves {
            sw.elapsed_ms += dt;
        }
        self.shockwaves.retain(|sw| sw.elapsed_ms < sw.duration_ms);

        // ── Sparks ──────────────────────────────────────────────────────
        for spark in &mut self.sparks {
            spark.remaining_ms -= dt;
            spark.intensity *= 0.97_f32.powf(dt / 50.0);
        }
        self.sparks.retain(|s| s.remaining_ms > 0.0);

        // ── Field lines (ambient) ───────────────────────────────────────
        let field_brightness_mult = 1.0_f32;
        let height = area.height as f32;

        for fl in &mut self.field_lines {
            fl.y += fl.speed * (dt / 1000.0);
            if fl.y > height + fl.length as f32 {
                fl.y = -(fl.length as f32);
                fl.x = self.rng.next_u32_range(0, area.width.max(1) as u32) as u16;
                fl.speed = self.rng.next_range(0.5, 1.5);
                fl.brightness = self.rng.next_range(0.35, 0.75) * field_brightness_mult;
            }
            fl.brightness = (fl.brightness / field_brightness_mult.max(0.01)).clamp(0.35, 0.75)
                * field_brightness_mult;
        }

        // ── Micro-pulses (ambient) ─────────────────────────────────────
        self.ambient_timer += dt;
        if height > 0.0 {
            let interval = self.rng.next_range(3000.0, 5000.0);
            if self.ambient_timer >= interval {
                self.ambient_timer = 0.0;
                let row = self.rng.next_u32_range(0, area.height.max(1) as u32) as u16;
                self.micro_pulses.push(MicroPulse {
                    row,
                    elapsed_ms: 0.0,
                    duration_ms: 400.0,
                });
            }
        }
        for mp in &mut self.micro_pulses {
            mp.elapsed_ms += dt;
        }
        self.micro_pulses
            .retain(|mp| mp.elapsed_ms < mp.duration_ms);
    }

    /// Trigger a burst at the given absolute row index (within the room list).
    pub fn trigger_burst(&mut self, epicenter_row: u16) {
        // Shockwave
        self.shockwaves.push(Shockwave {
            epicenter_row,
            elapsed_ms: 0.0,
            duration_ms: 1400.0,
        });

        // Sparks: 8-15 random short-lived near the epicenter
        let count = self.rng.next_u32_range(25, 40);
        let area = self.last_area;
        let half_h = (area.height as f32 / 2.0) as i16;
        for _ in 0..count {
            let spark_x = if area.width > 0 {
                self.rng.next_u32_range(0, area.width as u32) as u16
            } else {
                0
            };
            let dy = self.rng.next_range(-(half_h as f32), half_h as f32) as i16;
            let spark_y = (epicenter_row as i16 + dy).max(0) as u16;
            let char_idx = self.rng.next_u32_range(0, SPARK_CHARS.len() as u32) as usize;
            let lifetime = self.rng.next_range(300.0, 700.0);

            self.sparks.push(Spark {
                x: spark_x,
                y: spark_y,
                char_idx,
                remaining_ms: lifetime,
                intensity: 1.0,
            });
        }
    }

    /// Render EMP effects into the given buffer.
    /// `scroll_offset` maps absolute row indices to visual positions.
    pub fn render(&self, buf: &mut Buffer, area: Rect, scroll_offset: usize) {
        if area.width == 0 || area.height == 0 {
            return;
        }

        // Layer 1: Field lines
        self.render_field_lines(buf, area);

        // Layer 2: Micro-pulses
        self.render_micro_pulses(buf, area);

        // Layer 3: Shockwave rings
        self.render_shockwaves(buf, area, scroll_offset);

        // Layer 4: Sparks
        self.render_sparks(buf, area, scroll_offset);
    }

    // ── Private render layers ───────────────────────────────────────────────

    fn render_field_lines(&self, buf: &mut Buffer, area: Rect) {
        for fl in &self.field_lines {
            let x = area.x + fl.x;
            if x >= area.x + area.width {
                continue;
            }

            let brightness = fl.brightness;
            let r = 0u8;
            let g = (255.0 * brightness) as u8;
            let b = (255.0 * brightness) as u8;
            let color = Color::Rgb(r, g, b);
            let style = Style::default().fg(color);

            for i in 0..fl.length {
                let row = fl.y as i32 + i as i32;
                if row < area.y as i32 || row >= (area.y + area.height) as i32 {
                    continue;
                }
                let y = row as u16;
                let ch = FIELD_CHARS[i as usize % FIELD_CHARS.len()];
                let cell = &mut buf[(x, y)];
                cell.set_char(ch);
                cell.set_style(style);
            }
        }
    }

    fn render_micro_pulses(&self, buf: &mut Buffer, area: Rect) {
        for mp in &self.micro_pulses {
            let row = area.y + mp.row;
            if row >= area.y + area.height {
                continue;
            }

            // Sweep progress: 0.0 → 1.0
            let progress = mp.elapsed_ms / mp.duration_ms;
            let sweep_x = (progress * area.width as f32) as u16;

            // Shimmer band width: ~7 cells
            let band_width = 7u16;
            let start_x = sweep_x.saturating_sub(band_width / 2);
            let end_x = (sweep_x + band_width / 2 + 1).min(area.width);

            for dx in start_x..end_x {
                let x = area.x + dx;
                if x >= area.x + area.width {
                    break;
                }

                let dist = (dx as f32 - sweep_x as f32).abs() / (band_width as f32 / 2.0);
                let intensity = (1.0 - dist).max(0.0) * (1.0 - progress);
                let r = (255.0 * intensity) as u8;
                let g = 0u8;
                let b = (255.0 * intensity) as u8;
                let color = Color::Rgb(r, g, b);

                let cell = &mut buf[(x, row)];
                cell.set_char('⠒');
                cell.set_style(Style::default().fg(color));
            }
        }
    }

    fn render_shockwaves(&self, buf: &mut Buffer, area: Rect, scroll_offset: usize) {
        for sw in &self.shockwaves {
            let progress = sw.elapsed_ms / sw.duration_ms;
            // Exponential decay for intensity — gentle falloff for full-pane travel
            let intensity = (-progress).exp();
            if intensity < 0.02 {
                continue;
            }

            // Radius expands over time (full pane height so wave reaches both edges)
            let max_radius = (area.height as f32).max(4.0);
            let radius = progress * max_radius;

            // Visual epicenter (absolute row → screen position)
            let visual_epi = sw.epicenter_row as i32 - scroll_offset as i32 + area.y as i32 + 1; // +1 for border

            // Draw wave rings at epicenter ± radius
            for sign in [-1.0_f32, 1.0] {
                let wave_y = visual_epi as f32 + sign * radius;
                let wy = wave_y.round() as i32;
                if wy < area.y as i32 || wy >= (area.y + area.height) as i32 {
                    continue;
                }

                // Distance from epicenter determines character and color
                let dist = (wy as f32 - visual_epi as f32).abs() / max_radius;
                let char_idx =
                    ((dist * (WAVE_CHARS.len() - 1) as f32) as usize).min(WAVE_CHARS.len() - 1);
                let ch = WAVE_CHARS[char_idx];

                // 2-zone color: cyan core → magenta fringe
                let (cr, cg, cb) = {
                    // Core: cyan (0, 255, 255) → magenta (255, 0, 255)
                    let t = dist.min(1.0);
                    (255.0 * t, 255.0 * (1.0 - t), 255.0)
                };
                let r = (cr * intensity) as u8;
                let g = (cg * intensity) as u8;
                let b = (cb * intensity) as u8;
                let color = Color::Rgb(r, g, b);
                let style = Style::default().fg(color);

                // Draw across the width with some variation
                for dx in 0..area.width {
                    let x = area.x + dx;
                    let cell = &mut buf[(x, wy as u16)];
                    cell.set_char(ch);
                    cell.set_style(style);
                }
            }

            // Also render the epicenter row itself during early burst
            if progress < 0.3 {
                let epi_y = visual_epi;
                if epi_y >= area.y as i32 && epi_y < (area.y + area.height) as i32 {
                    let bright = intensity * (1.0 - progress / 0.3);
                    // Hot magenta-white flash
                    let r = (255.0 * bright) as u8;
                    let g = (50.0 * bright) as u8;
                    let b = (255.0 * bright) as u8;
                    let color = Color::Rgb(r, g, b);
                    let style = Style::default().fg(color);
                    for dx in 0..area.width {
                        let x = area.x + dx;
                        let cell = &mut buf[(x, epi_y as u16)];
                        cell.set_char('━');
                        cell.set_style(style);
                    }
                }
            }
        }
    }

    fn render_sparks(&self, buf: &mut Buffer, area: Rect, scroll_offset: usize) {
        for spark in &self.sparks {
            let x = area.x + spark.x;
            let visual_y = spark.y as i32 - scroll_offset as i32 + area.y as i32 + 1; // +1 for border

            if x >= area.x + area.width
                || visual_y < area.y as i32
                || visual_y >= (area.y + area.height) as i32
            {
                continue;
            }

            let ch = SPARK_CHARS[spark.char_idx % SPARK_CHARS.len()];
            let intensity = spark.intensity;

            // Alternate magenta-white and cyan-white sparks
            let (r, g, b) = if spark.char_idx % 2 == 0 {
                (
                    (255.0 * intensity) as u8,
                    (30.0 * intensity) as u8,
                    (255.0 * intensity) as u8,
                )
            } else {
                (
                    (30.0 * intensity) as u8,
                    (255.0 * intensity) as u8,
                    (255.0 * intensity) as u8,
                )
            };
            let color = Color::Rgb(r, g, b);

            let cell = &mut buf[(x, visual_y as u16)];
            cell.set_char(ch);
            cell.set_style(Style::default().fg(color));
        }
    }

    // ── Private helpers ─────────────────────────────────────────────────────

    fn init_field_lines(&mut self, area: Rect) {
        self.field_lines.clear();
        if area.width == 0 || area.height == 0 {
            return;
        }

        let count = self.rng.next_u32_range(5, 9);
        for _ in 0..count {
            let x = self.rng.next_u32_range(0, area.width as u32) as u16;
            let y = self.rng.next_range(0.0, area.height as f32);
            let length = self.rng.next_u32_range(3, area.height.max(4) as u32) as u16;
            let speed = self.rng.next_range(0.5, 1.5);
            let brightness = self.rng.next_range(0.35, 0.75);

            self.field_lines.push(FieldLine {
                x,
                y,
                length,
                speed,
                brightness,
            });
        }
    }
}
