use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::Frame;

use crate::ui::effects::Xorshift64;
use crate::ui::theme;
use crate::voip::{CallInfo, CallState};

const POPUP_WIDTH: u16 = 52;
const POPUP_HEIGHT: u16 = 13;
const WAVEFORM_LEN: usize = 38;

const SCRAMBLE_CHARS: &[char] = &[
    '░', '▒', '▓', '╳', '◊', 'ア', 'イ', 'ウ', 'エ', '0', '1', 'F', 'X', '█', '▌', '╌',
];
const WAVEFORM_CHARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

const REVEAL_MS_PER_CHAR: u64 = 40;
const SCRAMBLE_PHASE_MS: u64 = 300;

pub struct TransmissionPopup {
    rng: Xorshift64,
    reveal_elapsed_ms: u64,
    last_state: Option<CallState>,
    scramble_seed: u64,
    pulse_phase: f32,
    waveform: [f32; WAVEFORM_LEN],
    waveform_targets: [f32; WAVEFORM_LEN],
    waveform_phase: f32,
    connecting_dots: u8,
    connecting_accum_ms: u64,
    progress_bar_pos: u16,
    progress_accum_ms: u64,
}

impl TransmissionPopup {
    pub fn new() -> Self {
        Self {
            rng: Xorshift64::new(0xCAFE_BABE_DEAD_BEEF),
            reveal_elapsed_ms: 0,
            last_state: None,
            scramble_seed: 0,
            pulse_phase: 0.0,
            waveform: [0.0; WAVEFORM_LEN],
            waveform_targets: [0.0; WAVEFORM_LEN],
            waveform_phase: 0.0,
            connecting_dots: 0,
            connecting_accum_ms: 0,
            progress_bar_pos: 0,
            progress_accum_ms: 0,
        }
    }

    pub fn tick(&mut self, dt_ms: u64, call_state: &CallState) {
        // Reset reveal on state change
        if self.last_state.as_ref() != Some(call_state) {
            self.reveal_elapsed_ms = 0;
            self.last_state = Some(call_state.clone());
        } else {
            self.reveal_elapsed_ms += dt_ms;
        }

        // Pre-compute scramble seed for render purity
        self.scramble_seed = self.rng.next();

        // Pulse phase — speed varies by state
        let pulse_period_ms = match call_state {
            CallState::Inviting => 2000.0,
            CallState::Ringing => 800.0,
            CallState::Connecting => 600.0,
            CallState::Active => 4000.0,
        };
        self.pulse_phase += (dt_ms as f32 / pulse_period_ms) * std::f32::consts::TAU;
        if self.pulse_phase > std::f32::consts::TAU {
            self.pulse_phase -= std::f32::consts::TAU;
        }

        // Waveform
        self.waveform_phase += dt_ms as f32 * 0.003;
        let amplitude = match call_state {
            CallState::Active => 1.0,
            CallState::Connecting => 0.7,
            CallState::Ringing | CallState::Inviting => 0.3,
        };
        self.generate_waveform_targets(amplitude);
        for i in 0..WAVEFORM_LEN {
            self.waveform[i] += (self.waveform_targets[i] - self.waveform[i]) * 0.3;
        }

        // Cycle dots (0..3) every 500ms
        self.connecting_accum_ms += dt_ms;
        if self.connecting_accum_ms >= 500 {
            self.connecting_accum_ms -= 500;
            self.connecting_dots = (self.connecting_dots + 1) % 4;
        }

        // Slide progress bar packet every 80ms
        self.progress_accum_ms += dt_ms;
        if self.progress_accum_ms >= 80 {
            self.progress_accum_ms -= 80;
            self.progress_bar_pos = self.progress_bar_pos.wrapping_add(1);
        }
    }

    fn generate_waveform_targets(&mut self, amplitude: f32) {
        for i in 0..WAVEFORM_LEN {
            let x = i as f32 / WAVEFORM_LEN as f32;
            let p = self.waveform_phase;
            let wave1 = (x * std::f32::consts::TAU * 2.0 + p).sin() * 0.5;
            let wave2 = (x * std::f32::consts::TAU * 3.5 + p * 1.7).sin() * 0.3;
            let wave3 = (x * std::f32::consts::TAU * 5.0 + p * 0.6).sin() * 0.2;
            let noise = self.rng.next_f32() * 0.15 - 0.075;
            self.waveform_targets[i] =
                ((wave1 + wave2 + wave3 + noise + 0.5) * amplitude).clamp(0.05, 1.0);
        }
    }

    fn render_popup(&self, info: &CallInfo, frame: &mut Frame) {
        let area = frame.area();
        if area.width < 20 || area.height < 14 {
            return;
        }

        let popup_w = POPUP_WIDTH.min(area.width.saturating_sub(4));
        let show_waveform = area.height >= POPUP_HEIGHT + 4;
        let popup_h = if show_waveform {
            POPUP_HEIGHT
        } else {
            POPUP_HEIGHT - 3
        }
        .min(area.height.saturating_sub(4));

        let popup = centered_rect(popup_w, popup_h, area);
        let buf = frame.buffer_mut();
        let bounds = *buf.area();

        // Fill background
        for y in popup.y..popup.y + popup.height {
            for x in popup.x..popup.x + popup.width {
                if in_bounds(x, y, &bounds) {
                    buf[(x, y)].set_char(' ');
                    buf[(x, y)].set_style(Style::default().bg(theme::BG));
                }
            }
        }

        let border_color = self.pulse_color(&info.state);
        self.render_border(buf, &bounds, popup, border_color);
        self.render_title(buf, &bounds, popup, border_color, &info.state);
        self.render_caller_line(buf, &bounds, popup, info);
        self.render_separator(buf, &bounds, popup);
        self.render_state_content(buf, &bounds, popup, info);
        if show_waveform {
            self.render_waveform(buf, &bounds, popup, &info.state);
        }
        self.render_hints(buf, &bounds, popup, &info.state);
    }

    fn pulse_color(&self, state: &CallState) -> Color {
        let base = match state {
            CallState::Inviting | CallState::Connecting => theme::CYAN,
            CallState::Ringing | CallState::Active => theme::GREEN,
        };
        let brightness = (self.pulse_phase.sin() + 1.0) / 2.0;
        let factor = 0.35 + brightness * 0.65;
        if let Color::Rgb(r, g, b) = base {
            Color::Rgb(
                (r as f32 * factor) as u8,
                (g as f32 * factor) as u8,
                (b as f32 * factor) as u8,
            )
        } else {
            base
        }
    }

    fn render_border(&self, buf: &mut Buffer, bounds: &Rect, area: Rect, color: Color) {
        let s = Style::default().fg(color).bg(theme::BG);
        let x1 = area.x;
        let x2 = area.x + area.width - 1;
        let y1 = area.y;
        let y2 = area.y + area.height - 1;

        // Corners
        set_cell(buf, bounds, x1, y1, '╔', s);
        set_cell(buf, bounds, x2, y1, '╗', s);
        set_cell(buf, bounds, x1, y2, '╚', s);
        set_cell(buf, bounds, x2, y2, '╝', s);

        // Horizontals
        for x in (x1 + 1)..x2 {
            set_cell(buf, bounds, x, y1, '═', s);
            set_cell(buf, bounds, x, y2, '═', s);
        }

        // Verticals
        for y in (y1 + 1)..y2 {
            set_cell(buf, bounds, x1, y, '║', s);
            set_cell(buf, bounds, x2, y, '║', s);
        }

        // Decorative bottom glyph ◈
        let gx = x2.saturating_sub(5);
        if gx > x1 {
            set_cell(buf, bounds, gx, y2, '◈', s);
        }
    }

    fn render_title(
        &self,
        buf: &mut Buffer,
        bounds: &Rect,
        area: Rect,
        color: Color,
        state: &CallState,
    ) {
        let title = match state {
            CallState::Inviting => "OUTGOING TRANSMISSION",
            CallState::Ringing => "INCOMING TRANSMISSION",
            CallState::Connecting => "ESTABLISHING LINK",
            CallState::Active => "TRANSMISSION ACTIVE",
        };
        let title_chars: Vec<char> = title.chars().collect();

        let border_s = Style::default().fg(color).bg(theme::BG);
        let title_s = border_s.add_modifier(Modifier::BOLD);

        // ╔══╡ TITLE ╞═══...╗
        let bracket_l = area.x + 3;
        let title_start = bracket_l + 2;

        set_cell(buf, bounds, bracket_l, area.y, '╡', border_s);
        set_cell(buf, bounds, bracket_l + 1, area.y, ' ', border_s);

        // Text reveal effect
        let mut scramble_rng = Xorshift64::new(self.scramble_seed);
        for (i, &ch) in title_chars.iter().enumerate() {
            let x = title_start + i as u16;
            if x >= area.x + area.width - 1 {
                break;
            }
            let revealed = self.reveal_elapsed_ms >= SCRAMBLE_PHASE_MS
                && (i as u64) < (self.reveal_elapsed_ms - SCRAMBLE_PHASE_MS) / REVEAL_MS_PER_CHAR;
            let display = if revealed {
                ch
            } else {
                SCRAMBLE_CHARS[(scramble_rng.next() % SCRAMBLE_CHARS.len() as u64) as usize]
            };
            set_cell(buf, bounds, x, area.y, display, title_s);
        }

        let bracket_r_space = title_start + title_chars.len() as u16;
        let bracket_r = bracket_r_space + 1;
        set_cell(buf, bounds, bracket_r_space, area.y, ' ', border_s);
        if bracket_r < area.x + area.width - 1 {
            set_cell(buf, bounds, bracket_r, area.y, '╞', border_s);
        }
    }

    fn render_caller_line(
        &self,
        buf: &mut Buffer,
        bounds: &Rect,
        area: Rect,
        info: &CallInfo,
    ) {
        let row = area.y + 2;
        let left = area.x + 3;
        let right = area.x + area.width.saturating_sub(3);

        let color = match info.state {
            CallState::Ringing | CallState::Active => theme::GREEN,
            _ => theme::CYAN,
        };

        // ▶ username
        let name_s = Style::default()
            .fg(color)
            .bg(theme::BG)
            .add_modifier(Modifier::BOLD);
        set_cell(
            buf,
            bounds,
            left,
            row,
            '▶',
            Style::default().fg(color).bg(theme::BG),
        );
        set_cell(
            buf,
            bounds,
            left + 1,
            row,
            ' ',
            Style::default().bg(theme::BG),
        );
        write_str(buf, bounds, left + 2, row, &info.remote_user, name_s);

        // ◉ VOICE right-aligned
        let voice = "◉ VOICE";
        let vx = right.saturating_sub(voice.chars().count() as u16);
        write_str(
            buf,
            bounds,
            vx,
            row,
            voice,
            Style::default().fg(theme::DIM).bg(theme::BG),
        );
    }

    fn render_separator(&self, buf: &mut Buffer, bounds: &Rect, area: Rect) {
        let row = area.y + 3;
        let left = area.x + 3;
        let right = area.x + area.width.saturating_sub(3);
        let s = Style::default().fg(theme::DIM).bg(theme::BG);
        for x in left..right {
            set_cell(buf, bounds, x, row, '┄', s);
        }
    }

    fn render_state_content(
        &self,
        buf: &mut Buffer,
        bounds: &Rect,
        area: Rect,
        info: &CallInfo,
    ) {
        let row = area.y + 5;
        let left = area.x + 3;
        let right = area.x + area.width.saturating_sub(3);

        match info.state {
            CallState::Inviting => {
                let dots = ".".repeat((self.connecting_dots % 3) as usize + 1);
                let text = format!("DIALING{}", dots);
                write_str(
                    buf,
                    bounds,
                    left,
                    row,
                    &text,
                    Style::default().fg(theme::CYAN).bg(theme::BG),
                );
            }
            CallState::Ringing => {
                let s = Style::default().fg(theme::GREEN).bg(theme::BG);
                write_str(buf, bounds, left, row, "SIGNAL DETECTED", s);
                write_str(buf, bounds, left, row + 1, "AWAITING RESPONSE", s);
            }
            CallState::Connecting => {
                let texts = [
                    "NEGOTIATING HANDSHAKE",
                    "EXCHANGING KEYS",
                    "ROUTING SIGNAL",
                ];
                let idx = (self.connecting_dots as usize) % texts.len();
                write_str(
                    buf,
                    bounds,
                    left,
                    row,
                    texts[idx],
                    Style::default().fg(theme::CYAN).bg(theme::BG),
                );

                // Progress bar with sliding packet
                let bar_w = (right - left) as usize;
                if bar_w > 0 {
                    let pos = self.progress_bar_pos as usize % bar_w;
                    let bar_s = Style::default().fg(theme::CYAN).bg(theme::BG);
                    for i in 0..bar_w {
                        let ch = if i == pos { '╸' } else { '━' };
                        set_cell(buf, bounds, left + i as u16, row + 1, ch, bar_s);
                    }
                    // Bright packet highlight
                    let px = left + pos as u16;
                    if in_bounds(px, row + 1, bounds) {
                        buf[(px, row + 1)].set_style(
                            Style::default()
                                .fg(Color::Rgb(255, 255, 255))
                                .bg(theme::BG),
                        );
                    }
                }
            }
            CallState::Active => {
                let elapsed = info.elapsed_display();
                let text = format!("◧ VOICE ━━━━━━━ {}", elapsed);
                write_str(
                    buf,
                    bounds,
                    left,
                    row,
                    &text,
                    Style::default().fg(theme::GREEN).bg(theme::BG),
                );
            }
        }
    }

    fn render_waveform(
        &self,
        buf: &mut Buffer,
        bounds: &Rect,
        area: Rect,
        state: &CallState,
    ) {
        let top = area.y + 8;
        let mid = area.y + 9;
        let bot = area.y + 10;
        let left = area.x + 3;
        let right = area.x + area.width.saturating_sub(4);

        if bot >= area.y + area.height {
            return;
        }

        let dim = Style::default().fg(theme::DIM).bg(theme::BG);

        // Top: ┌╌╌...╌┐
        set_cell(buf, bounds, left, top, '┌', dim);
        for x in (left + 1)..right {
            set_cell(buf, bounds, x, top, '╌', dim);
        }
        set_cell(buf, bounds, right, top, '┐', dim);

        // Bottom: └╌╌...╌┘
        set_cell(buf, bounds, left, bot, '└', dim);
        for x in (left + 1)..right {
            set_cell(buf, bounds, x, bot, '╌', dim);
        }
        set_cell(buf, bounds, right, bot, '┘', dim);

        // Content: ╎ waveform ╎
        set_cell(buf, bounds, left, mid, '╎', dim);
        set_cell(
            buf,
            bounds,
            left + 1,
            mid,
            ' ',
            Style::default().bg(theme::BG),
        );

        let wave_color = match state {
            CallState::Active | CallState::Ringing => theme::GREEN,
            _ => theme::CYAN,
        };
        let ws = Style::default().fg(wave_color).bg(theme::BG);
        let wave_x = left + 2;
        for i in 0..WAVEFORM_LEN {
            let x = wave_x + i as u16;
            if x >= right {
                break;
            }
            let idx = ((self.waveform[i] * 7.0).round() as usize).min(7);
            set_cell(buf, bounds, x, mid, WAVEFORM_CHARS[idx], ws);
        }

        let after = wave_x + WAVEFORM_LEN as u16;
        if after < right {
            set_cell(
                buf,
                bounds,
                after,
                mid,
                ' ',
                Style::default().bg(theme::BG),
            );
        }
        set_cell(buf, bounds, right, mid, '╎', dim);
    }

    fn render_hints(
        &self,
        buf: &mut Buffer,
        bounds: &Rect,
        area: Rect,
        state: &CallState,
    ) {
        let row = area.y + area.height - 2;

        let segments: &[(&str, Color, bool)] = match state {
            CallState::Inviting | CallState::Connecting | CallState::Active => {
                &[("c ", theme::CYAN, true), (":hangup", theme::RED, true)]
            }
            CallState::Ringing => &[
                ("a ", theme::CYAN, true),
                (":answer", theme::CYAN, true),
                (" │ ", theme::DIM, false),
                ("r ", theme::RED, true),
                (":reject", theme::RED, true),
            ],
        };

        let total: usize = segments.iter().map(|(t, _, _)| t.chars().count()).sum();
        let inner = area.width.saturating_sub(2) as usize;
        let offset = inner.saturating_sub(total) / 2;
        let mut x = area.x + 1 + offset as u16;

        for &(text, color, bold) in segments {
            let mut s = Style::default().fg(color).bg(theme::BG);
            if bold {
                s = s.add_modifier(Modifier::BOLD);
            }
            write_str(buf, bounds, x, row, text, s);
            x += text.chars().count() as u16;
        }
    }
}

// ── helpers ──────────────────────────────────────────

fn centered_rect(w: u16, h: u16, area: Rect) -> Rect {
    Rect::new(
        area.x + area.width.saturating_sub(w) / 2,
        area.y + area.height.saturating_sub(h) / 2,
        w.min(area.width),
        h.min(area.height),
    )
}

#[inline]
fn in_bounds(x: u16, y: u16, r: &Rect) -> bool {
    x >= r.x && x < r.x + r.width && y >= r.y && y < r.y + r.height
}

#[inline]
fn set_cell(buf: &mut Buffer, bounds: &Rect, x: u16, y: u16, ch: char, style: Style) {
    if in_bounds(x, y, bounds) {
        let cell = &mut buf[(x, y)];
        cell.set_char(ch);
        cell.set_style(style);
    }
}

fn write_str(buf: &mut Buffer, bounds: &Rect, x: u16, y: u16, text: &str, style: Style) {
    for (i, ch) in text.chars().enumerate() {
        set_cell(buf, bounds, x + i as u16, y, ch, style);
    }
}

/// Public entry point for ui::mod to call
pub fn render(popup: &TransmissionPopup, info: &CallInfo, frame: &mut Frame) {
    popup.render_popup(info, frame);
}
