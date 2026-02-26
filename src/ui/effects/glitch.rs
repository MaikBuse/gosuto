use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use super::Xorshift64;

const GLITCH_CHARS: &[char] = &[
    '░', '▒', '▓', '█', '╌', '╍', '┄', '┅', '▌', '▐', '▀', '▄', '⣿', '⡟', '⠿', '⣏', '¦',
    '÷', '±', '¬',
];

const CYAN: Color = Color::Rgb(0x00, 0xFF, 0xFF);
const MAGENTA: Color = Color::Rgb(0xFF, 0x00, 0xFF);

struct GlitchBand {
    y_offset: u16,
    height: u16,
    shift: i16,
    tint: Color,
    remaining_ms: u32,
    corrupt_count: u8,
    corrupt_positions: [u16; 3],
}

pub struct GlitchEffect {
    rng: Xorshift64,
    cooldown_ms: u32,
    bands: Vec<GlitchBand>,
}

impl GlitchEffect {
    pub fn new() -> Self {
        let mut rng = Xorshift64::new(0x6117_C43D_C0FF_EE42);
        let cooldown = rng.next_u32_range(2000, 6000);
        Self {
            rng,
            cooldown_ms: cooldown,
            bands: Vec::new(),
        }
    }

    pub fn tick(&mut self, dt_ms: u64, max_height: u16) {
        // Decrement band lifetimes, remove expired
        let dt = dt_ms as u32;
        self.bands.retain_mut(|band| {
            band.remaining_ms = band.remaining_ms.saturating_sub(dt);
            band.remaining_ms > 0
        });

        // Decrement cooldown, spawn new bands when it hits 0
        self.cooldown_ms = self.cooldown_ms.saturating_sub(dt);
        if self.cooldown_ms == 0 && max_height > 0 {
            let band_count = self.rng.next_u32_range(1, 5);
            for _ in 0..band_count {
                self.spawn_band(max_height);
            }
            self.cooldown_ms = self.rng.next_u32_range(2000, 6000);
        }
    }

    fn spawn_band(&mut self, max_height: u16) {
        let y_offset = self.rng.next_u32_range(0, max_height as u32) as u16;
        let height = self.rng.next_u32_range(1, 4).min(max_height as u32 - y_offset as u32) as u16;
        if height == 0 {
            return;
        }

        // shift: -3..=3, never 0
        let shift_mag = self.rng.next_u32_range(1, 4) as i16;
        let shift = if self.rng.next().is_multiple_of(2) {
            shift_mag
        } else {
            -shift_mag
        };

        let tint = if self.rng.next().is_multiple_of(2) {
            CYAN
        } else {
            MAGENTA
        };

        let remaining_ms = self.rng.next_u32_range(100, 250);
        let corrupt_count = self.rng.next_u32_range(0, 4) as u8;

        let mut corrupt_positions = [0u16; 3];
        for pos in corrupt_positions.iter_mut().take(corrupt_count as usize) {
            *pos = self.rng.next_u32_range(0, 200) as u16; // clamped to panel width at render time
        }

        self.bands.push(GlitchBand {
            y_offset,
            height,
            shift,
            tint,
            remaining_ms,
            corrupt_count,
            corrupt_positions,
        });
    }

    pub fn post_process(&self, buf: &mut Buffer, areas: &[Rect]) {
        if self.bands.is_empty() {
            return;
        }

        for area in areas {
            if area.width == 0 || area.height == 0 {
                continue;
            }
            for band in &self.bands {
                self.apply_band(buf, *area, band);
            }
        }
    }

    fn apply_band(&self, buf: &mut Buffer, area: Rect, band: &GlitchBand) {
        let buf_area = *buf.area();

        for dy in 0..band.height {
            let row = area.y + band.y_offset + dy;
            if row >= area.y + area.height || row >= buf_area.y + buf_area.height {
                break;
            }

            // Read entire row within panel bounds into temp vec (clone to release borrow)
            let row_cells: Vec<Cell> = (area.x..area.x + area.width)
                .filter(|&x| x < buf_area.x + buf_area.width)
                .map(|x| buf[(x, row)].clone())
                .collect();

            // Write cells back shifted by band.shift, clamped to panel bounds
            let shift = band.shift as i32;
            for (i, cell) in row_cells.into_iter().enumerate() {
                let dest_x = area.x as i32 + i as i32 + shift;
                if dest_x >= area.x as i32
                    && dest_x < (area.x + area.width) as i32
                    && (dest_x as u16) < buf_area.x + buf_area.width
                {
                    buf[(dest_x as u16, row)] = cell;
                }
            }

            // Fill gap cells with colored blocks
            let tint_style = Style::default().fg(band.tint).bg(band.tint);
            if shift > 0 {
                for gx in area.x..area.x + (shift as u16).min(area.width) {
                    if gx < buf_area.x + buf_area.width {
                        let cell = &mut buf[(gx, row)];
                        cell.set_char('▌');
                        cell.set_style(tint_style);
                    }
                }
            } else if shift < 0 {
                let abs_shift = (-shift) as u16;
                let gap_start = area.x + area.width.saturating_sub(abs_shift);
                for gx in gap_start..area.x + area.width {
                    if gx < buf_area.x + buf_area.width {
                        let cell = &mut buf[(gx, row)];
                        cell.set_char('▐');
                        cell.set_style(tint_style);
                    }
                }
            }

            // Corrupt random cells with glitch characters
            for ci in 0..band.corrupt_count as usize {
                let cx = area.x + (band.corrupt_positions[ci] % area.width);
                if cx < buf_area.x + buf_area.width {
                    let glyph_idx =
                        (band.corrupt_positions[ci] as usize + ci) % GLITCH_CHARS.len();
                    let cell = &mut buf[(cx, row)];
                    cell.set_char(GLITCH_CHARS[glyph_idx]);
                    cell.set_style(Style::default().fg(band.tint));
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn is_active(&self) -> bool {
        !self.bands.is_empty()
    }
}
