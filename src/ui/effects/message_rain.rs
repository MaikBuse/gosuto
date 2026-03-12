use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use super::Xorshift64;
use crate::ui::theme;

struct FallingCell {
    target_x: u16,
    target_y: u16,
    current_y: f32,
    speed: f32,
    delay_ms: f32,
    landed: bool,
    symbol: String,
    style: Style,
}

pub struct MessageRain {
    cells: Vec<FallingCell>,
    active: bool,
    elapsed_ms: f32,
    area: Rect,
    rng: Xorshift64,
}

impl MessageRain {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            active: false,
            elapsed_ms: 0.0,
            area: Rect::default(),
            rng: Xorshift64::new(0xCAFE_BABE_DEAD_BEEF),
        }
    }

    pub fn start(&mut self, snapshot: &Buffer, area: Rect) {
        self.cells.clear();
        self.elapsed_ms = 0.0;
        self.area = area;

        let width = area.width as f32;
        let height = area.height as f32;

        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                let cell = &snapshot[(x, y)];
                let sym = cell.symbol();
                if sym.trim().is_empty() {
                    continue;
                }

                let col_frac = (x - area.x) as f32 / width.max(1.0);
                let cascade_delay = col_frac * 400.0;
                let jitter = self.rng.next_range(0.0, 200.0);
                let delay = cascade_delay + jitter;

                let start_offset = self.rng.next_range(2.0, (height * 0.5).max(3.0));
                let start_y = area.y as f32 - start_offset;

                let speed = self.rng.next_range(25.0, 45.0);

                self.cells.push(FallingCell {
                    target_x: x,
                    target_y: y,
                    current_y: start_y,
                    speed,
                    delay_ms: delay,
                    landed: false,
                    symbol: sym.to_string(),
                    style: cell.style(),
                });
            }
        }

        self.active = !self.cells.is_empty();
    }

    pub fn tick(&mut self, dt_ms: u64, area: Rect) {
        if !self.active {
            return;
        }

        // Deactivate on resize
        if area != self.area {
            self.active = false;
            self.cells.clear();
            return;
        }

        self.elapsed_ms += dt_ms as f32;

        // Hard cap at 2000ms
        if self.elapsed_ms > 2000.0 {
            self.active = false;
            self.cells.clear();
            return;
        }

        let dt_sec = dt_ms as f32 / 1000.0;
        let mut all_landed = true;

        for cell in &mut self.cells {
            if cell.landed {
                continue;
            }

            if cell.delay_ms > 0.0 {
                cell.delay_ms -= dt_ms as f32;
                all_landed = false;
                continue;
            }

            cell.current_y += cell.speed * dt_sec;

            if cell.current_y >= cell.target_y as f32 {
                cell.current_y = cell.target_y as f32;
                cell.landed = true;
            } else {
                all_landed = false;
            }
        }

        if all_landed {
            self.active = false;
            self.cells.clear();
        }
    }

    pub fn render(&self, buf: &mut Buffer) {
        if !self.active {
            return;
        }

        // Clear the area to BG first
        for y in self.area.y..self.area.y + self.area.height {
            for x in self.area.x..self.area.x + self.area.width {
                let c = &mut buf[(x, y)];
                c.reset();
                c.set_style(Style::default().bg(theme::BG));
            }
        }

        for cell in &self.cells {
            if cell.delay_ms > 0.0 {
                continue;
            }

            let draw_y = cell.current_y as u16;

            if draw_y < self.area.y || draw_y >= self.area.y + self.area.height {
                continue;
            }
            if cell.target_x < self.area.x || cell.target_x >= self.area.x + self.area.width {
                continue;
            }

            let c = &mut buf[(cell.target_x, draw_y)];
            c.set_symbol(&cell.symbol);
            c.set_style(cell.style);
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn area(&self) -> Rect {
        self.area
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_inactive() {
        let rain = MessageRain::new();
        assert!(!rain.is_active());
    }

    #[test]
    fn start_with_empty_buffer_stays_inactive() {
        let mut rain = MessageRain::new();
        let area = Rect::new(0, 0, 10, 5);
        let buf = Buffer::empty(area);
        rain.start(&buf, area);
        assert!(!rain.is_active());
    }

    #[test]
    fn start_with_content_activates() {
        let mut rain = MessageRain::new();
        let area = Rect::new(0, 0, 10, 5);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)].set_symbol("A");
        rain.start(&buf, area);
        assert!(rain.is_active());
    }

    #[test]
    fn deactivates_on_resize() {
        let mut rain = MessageRain::new();
        let area = Rect::new(0, 0, 10, 5);
        let mut buf = Buffer::empty(area);
        buf[(0, 0)].set_symbol("A");
        rain.start(&buf, area);
        assert!(rain.is_active());

        let new_area = Rect::new(0, 0, 20, 10);
        rain.tick(16, new_area);
        assert!(!rain.is_active());
    }

    #[test]
    fn deactivates_after_duration_cap() {
        let mut rain = MessageRain::new();
        let area = Rect::new(0, 0, 10, 5);
        let mut buf = Buffer::empty(area);
        buf[(5, 4)].set_symbol("X");
        rain.start(&buf, area);
        assert!(rain.is_active());

        // Tick past the 2000ms cap
        rain.tick(2100, area);
        assert!(!rain.is_active());
    }

    #[test]
    fn cells_eventually_land() {
        let mut rain = MessageRain::new();
        let area = Rect::new(0, 0, 5, 5);
        let mut buf = Buffer::empty(area);
        buf[(2, 2)].set_symbol("Z");
        rain.start(&buf, area);

        // Tick many small steps — should eventually all land
        for _ in 0..200 {
            if !rain.is_active() {
                break;
            }
            rain.tick(16, area);
        }
        assert!(!rain.is_active());
    }
}
