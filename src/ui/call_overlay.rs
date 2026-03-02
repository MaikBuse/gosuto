use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use crate::ui::effects::{TextReveal, Xorshift64};
use crate::ui::icons::Icons;
use crate::ui::theme;
use crate::voip::{CallInfo, CallState};

const POPUP_WIDTH: u16 = 52;
const BASE_POPUP_HEIGHT: u16 = 13;
const WAVEFORM_LEN: usize = (POPUP_WIDTH - 9) as usize;

const WAVEFORM_CHARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Display state combines app-level ringing with CallState
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallDisplayState {
    Ringing,
    Connecting,
    Active,
}

pub struct TransmissionPopup {
    rng: Xorshift64,
    title_reveal: TextReveal,
    last_display_state: Option<CallDisplayState>,
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
            title_reveal: TextReveal::new(0xC0DE_CAFE_0003),
            last_display_state: None,
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

    pub fn tick(&mut self, dt_ms: u64, display_state: &CallDisplayState) {
        // Reset reveal on state change
        if self.last_display_state.as_ref() != Some(display_state) {
            self.title_reveal.trigger();
            self.last_display_state = Some(display_state.clone());
        }

        self.title_reveal.tick(dt_ms);

        // Pulse phase — speed varies by state
        let pulse_period_ms = match display_state {
            CallDisplayState::Ringing => 800.0,
            CallDisplayState::Connecting => 600.0,
            CallDisplayState::Active => 4000.0,
        };
        self.pulse_phase += (dt_ms as f32 / pulse_period_ms) * std::f32::consts::TAU;
        if self.pulse_phase > std::f32::consts::TAU {
            self.pulse_phase -= std::f32::consts::TAU;
        }

        // Waveform
        self.waveform_phase += dt_ms as f32 * 0.003;
        let amplitude = match display_state {
            CallDisplayState::Active => 1.0,
            CallDisplayState::Connecting => 0.7,
            CallDisplayState::Ringing => 0.3,
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

    fn render_popup(
        &self,
        info: &CallInfo,
        display_state: &CallDisplayState,
        icons: &Icons,
        frame: &mut Frame,
    ) {
        let area = frame.area();
        if area.width < 20 || area.height < 14 {
            return;
        }

        let popup_w = POPUP_WIDTH.min(area.width.saturating_sub(4));

        // Dynamic height for participant list in group calls
        let participant_lines = if info.participants.len() > 1 {
            info.participants.len() as u16
        } else {
            1 // single participant or joining
        };
        let room_name_line: u16 = if info.room_name.is_some() { 1 } else { 0 };
        let popup_height = BASE_POPUP_HEIGHT + participant_lines.saturating_sub(1) + room_name_line;

        let show_waveform = area.height >= popup_height + 4;
        let popup_h = if show_waveform {
            popup_height
        } else {
            popup_height - 3
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

        let border_color = self.pulse_color(display_state);
        self.render_border(buf, &bounds, popup, border_color);
        self.render_title(buf, &bounds, popup, border_color, display_state, info);
        self.render_caller_line(buf, &bounds, popup, info, display_state, icons);
        self.render_separator(buf, &bounds, popup, room_name_line);
        self.render_state_content(buf, &bounds, popup, info, display_state, room_name_line);
        if show_waveform {
            self.render_waveform(buf, &bounds, popup, display_state, room_name_line);
        }
        self.render_hints(buf, &bounds, popup, display_state);
    }

    fn pulse_color(&self, state: &CallDisplayState) -> Color {
        let base = match state {
            CallDisplayState::Connecting => theme::CYAN,
            CallDisplayState::Ringing | CallDisplayState::Active => theme::GREEN,
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
        state: &CallDisplayState,
        info: &CallInfo,
    ) {
        let title = match state {
            CallDisplayState::Ringing => "INCOMING TRANSMISSION",
            CallDisplayState::Connecting => "ESTABLISHING LINK",
            CallDisplayState::Active => {
                if info.participants.len() > 1 {
                    // Will format dynamically below
                    "TRANSMISSION ACTIVE"
                } else {
                    "TRANSMISSION ACTIVE"
                }
            }
        };
        let border_s = Style::default().fg(color).bg(theme::BG);
        let title_s = border_s.add_modifier(Modifier::BOLD);

        // ╔══╡ TITLE ╞═══...╗
        let bracket_l = area.x + 3;
        let title_start = bracket_l + 2;

        set_cell(buf, bounds, bracket_l, area.y, '╡', border_s);
        set_cell(buf, bounds, bracket_l + 1, area.y, ' ', border_s);

        // For active group calls, show participant count
        let display_title = if *state == CallDisplayState::Active && info.participants.len() > 1 {
            format!("{} ({})", title, info.participants.len())
        } else {
            title.to_string()
        };

        // Text reveal effect
        let revealed_chars = self.title_reveal.render_chars(&display_title);
        for (i, ch) in revealed_chars.into_iter().enumerate() {
            let x = title_start + i as u16;
            if x >= area.x + area.width - 1 {
                break;
            }
            set_cell(buf, bounds, x, area.y, ch, title_s);
        }

        let bracket_r_space = title_start + display_title.len() as u16;
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
        state: &CallDisplayState,
        icons: &Icons,
    ) {
        let row = area.y + 2;
        let left = area.x + 3;
        let right = area.x + area.width.saturating_sub(3);

        let color = match state {
            CallDisplayState::Ringing | CallDisplayState::Active => theme::GREEN,
            _ => theme::CYAN,
        };

        let name_s = Style::default()
            .fg(color)
            .bg(theme::BG)
            .add_modifier(Modifier::BOLD);

        // Room name line (above participant line)
        if let Some(ref name) = info.room_name {
            let room_s = Style::default().fg(theme::CYAN).bg(theme::BG);
            let label = format!("{} {}", icons.home, name);
            let max_w = (right - left) as usize;
            let truncated: String = label.chars().take(max_w).collect();
            write_str(buf, bounds, left, row, &truncated, room_s);
        }

        let caller_row = if info.room_name.is_some() {
            row + 1
        } else {
            row
        };

        if info.participants.is_empty() {
            // Joining, no participants yet
            write_str(
                buf,
                bounds,
                left,
                caller_row,
                icons.participant,
                Style::default().fg(color).bg(theme::BG),
            );
            set_cell(
                buf,
                bounds,
                left + 1,
                caller_row,
                ' ',
                Style::default().bg(theme::BG),
            );
            write_str(buf, bounds, left + 2, caller_row, "joining...", name_s);
        } else if info.participants.len() == 1 {
            // 1:1 call — show single participant
            write_str(
                buf,
                bounds,
                left,
                caller_row,
                icons.participant,
                Style::default().fg(color).bg(theme::BG),
            );
            set_cell(
                buf,
                bounds,
                left + 1,
                caller_row,
                ' ',
                Style::default().bg(theme::BG),
            );
            write_str(
                buf,
                bounds,
                left + 2,
                caller_row,
                &info.participants[0],
                name_s,
            );
        } else {
            // Group call — show participant list
            for (i, participant) in info.participants.iter().enumerate() {
                let y = caller_row + i as u16;
                if y >= area.y + area.height - 3 {
                    break;
                }
                write_str(
                    buf,
                    bounds,
                    left,
                    y,
                    icons.participant,
                    Style::default().fg(color).bg(theme::BG),
                );
                set_cell(
                    buf,
                    bounds,
                    left + 1,
                    y,
                    ' ',
                    Style::default().bg(theme::BG),
                );
                write_str(buf, bounds, left + 2, y, participant, name_s);
            }
        }

        // VOICE right-aligned
        let voice = format!("{} VOICE", icons.voice);
        let voice = voice.as_str();
        let vx = right.saturating_sub(voice.chars().count() as u16);
        write_str(
            buf,
            bounds,
            vx,
            caller_row,
            voice,
            Style::default().fg(theme::DIM).bg(theme::BG),
        );
    }

    fn render_separator(&self, buf: &mut Buffer, bounds: &Rect, area: Rect, y_offset: u16) {
        let row = area.y + 3 + y_offset;
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
        state: &CallDisplayState,
        y_offset: u16,
    ) {
        let row = area.y + 5 + y_offset;
        let left = area.x + 3;
        let right = area.x + area.width.saturating_sub(3);

        match state {
            CallDisplayState::Ringing => {
                let s = Style::default().fg(theme::GREEN).bg(theme::BG);
                write_str(buf, bounds, left, row, "SIGNAL DETECTED", s);
                write_str(buf, bounds, left, row + 1, "AWAITING RESPONSE", s);
            }
            CallDisplayState::Connecting => {
                let texts = ["NEGOTIATING HANDSHAKE", "EXCHANGING KEYS", "ROUTING SIGNAL"];
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
                            Style::default().fg(Color::Rgb(255, 255, 255)).bg(theme::BG),
                        );
                    }
                }
            }
            CallDisplayState::Active => {
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
        state: &CallDisplayState,
        y_offset: u16,
    ) {
        let top = area.y + 8 + y_offset;
        let mid = area.y + 9 + y_offset;
        let bot = area.y + 10 + y_offset;
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
            CallDisplayState::Active | CallDisplayState::Ringing => theme::GREEN,
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
            set_cell(buf, bounds, after, mid, ' ', Style::default().bg(theme::BG));
        }
        set_cell(buf, bounds, right, mid, '╎', dim);
    }

    fn render_hints(&self, buf: &mut Buffer, bounds: &Rect, area: Rect, state: &CallDisplayState) {
        let row = area.y + area.height - 2;

        let segments: &[(&str, Color, bool)] = match state {
            CallDisplayState::Connecting | CallDisplayState::Active => {
                &[("c ", theme::CYAN, true), (":hangup", theme::RED, true)]
            }
            CallDisplayState::Ringing => &[
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

/// Public entry point — render with appropriate display state
pub fn render(
    popup: &TransmissionPopup,
    info: &CallInfo,
    ds: &CallDisplayState,
    icons: &Icons,
    frame: &mut Frame,
) {
    popup.render_popup(info, ds, icons, frame);
}

/// Render for incoming ringing (no CallInfo yet)
pub fn render_ringing(
    popup: &TransmissionPopup,
    caller: &str,
    room_id: &str,
    room_name: Option<&str>,
    icons: &Icons,
    frame: &mut Frame,
) {
    // Create a temporary CallInfo for display
    let info = CallInfo {
        room_id: room_id.to_string(),
        room_name: room_name.map(|s| s.to_string()),
        state: CallState::Connecting, // doesn't matter, display_state overrides
        is_incoming: true,
        participants: vec![caller.to_string()],
        started_at: None,
    };
    popup.render_popup(&info, &CallDisplayState::Ringing, icons, frame);
}
