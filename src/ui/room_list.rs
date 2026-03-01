use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders},
};

use crate::app::App;
use crate::input::FocusPanel;
use crate::state::{DisplayRow, RoomCategory};
use crate::ui::theme;

pub struct RoomListAnimState {
    pub pulse_phase: f32,
    pub flash_timer: Option<u64>,
    pub flash_row: Option<usize>,
}

impl RoomListAnimState {
    pub fn new() -> Self {
        Self {
            pulse_phase: 0.0,
            flash_timer: None,
            flash_row: None,
        }
    }

    pub fn tick(&mut self, dt_ms: u64) {
        // Advance pulse phase — full cycle ~1600ms
        self.pulse_phase += (dt_ms as f32 / 1600.0) * std::f32::consts::TAU;
        if self.pulse_phase > std::f32::consts::TAU {
            self.pulse_phase -= std::f32::consts::TAU;
        }

        // Decay flash timer
        if let Some(ref mut remaining) = self.flash_timer {
            if *remaining <= dt_ms {
                self.flash_timer = None;
                self.flash_row = None;
            } else {
                *remaining -= dt_ms;
            }
        }
    }

    pub fn trigger_flash(&mut self, row: usize) {
        self.flash_timer = Some(250);
        self.flash_row = Some(row);
    }

    fn pulse_style(&self) -> Style {
        let t = (self.pulse_phase.sin() + 1.0) / 2.0;
        let shimmer = ((self.pulse_phase * 3.7).sin() + 1.0) / 2.0;
        let factor = 0.45 + (t + shimmer * 0.08) * 0.55;

        Style::default()
            .fg(Color::Rgb(0, 0, 0))
            .bg(Color::Rgb(
                (20.0 * factor) as u8,
                (255.0 * factor) as u8,
                (255.0 * factor) as u8,
            ))
            .add_modifier(Modifier::BOLD)
    }

    fn flash_style(&self) -> Option<Style> {
        let remaining = self.flash_timer?;
        let intensity = (remaining as f32 / 250.0).powi(2);

        Some(
            Style::default()
                .fg(Color::Rgb(
                    (255.0 * (1.0 - intensity)) as u8,
                    (255.0 * (1.0 - intensity)) as u8,
                    (255.0 * (1.0 - intensity)) as u8,
                ))
                .bg(Color::Rgb((255.0 * intensity) as u8, 255, 255))
                .add_modifier(Modifier::BOLD),
        )
    }

    fn row_style(&self, row_idx: usize, is_selected: bool, focused: bool) -> Option<Style> {
        if !is_selected {
            return None;
        }
        if focused {
            if self.flash_row == Some(row_idx)
                && let Some(flash) = self.flash_style()
            {
                return Some(flash);
            }
            Some(self.pulse_style())
        } else {
            // Static cyan highlight when panel is not focused
            Some(
                Style::default()
                    .fg(Color::Rgb(0, 0, 0))
                    .bg(theme::CYAN)
                    .add_modifier(Modifier::BOLD),
            )
        }
    }
}

/// Compute the scroll offset for the room list given the current state and area.
pub fn scroll_offset(app: &App, area: Rect) -> usize {
    let inner_height = area.height.saturating_sub(2) as usize; // subtract borders
    let total_rows = app.room_list.display_rows.len();
    let selected = app.room_list.selected;

    if total_rows <= inner_height || selected < inner_height / 2 {
        0
    } else if selected > total_rows - inner_height / 2 {
        total_rows - inner_height
    } else {
        selected - inner_height / 2
    }
}

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.vim.focus == FocusPanel::RoomList;

    let border_style = if focused {
        theme::border_focused_style()
    } else {
        theme::border_style()
    };

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            " ROOMS ",
            theme::title_style(),
        )]))
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(theme::BG));

    frame.render_widget(block, area);

    // Inner area (inside borders)
    let inner = Rect::new(
        area.x + 1,
        area.y + 1,
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    );

    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let display_rows = &app.room_list.display_rows;
    let selected = app.room_list.selected;
    let anim = &app.room_list_anim;
    let visible_height = inner.height as usize;
    let total_rows = display_rows.len();

    // Scroll offset to keep selected row centered
    let scroll_offset = if total_rows <= visible_height || selected < visible_height / 2 {
        0
    } else if selected > total_rows - visible_height / 2 {
        total_rows - visible_height
    } else {
        selected - visible_height / 2
    };

    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    for (vi, row_idx) in (scroll_offset..display_rows.len()).enumerate() {
        if vi >= visible_height {
            break;
        }

        let y = inner.y + vi as u16;
        let row = &display_rows[row_idx];
        let is_selected = row_idx == selected;

        match row {
            DisplayRow::SectionHeader { label } => {
                let style = Style::default()
                    .fg(theme::DIM)
                    .bg(theme::BG)
                    .add_modifier(Modifier::BOLD);
                let text = format!(" {} ", label);
                write_str_clipped(buf, &bounds, inner.x + 1, y, &text, style, &inner);
                // Fill remaining with ─
                let line_style = Style::default().fg(theme::DIM).bg(theme::BG);
                let text_end = inner.x + 1 + text.len() as u16;
                for x in text_end..inner.x + inner.width {
                    set_cell_if(buf, &bounds, x, y, '─', line_style);
                }
            }
            DisplayRow::SpaceHeader {
                name,
                collapsed,
                unread_count,
                ..
            } => {
                let arrow = if *collapsed { "▶" } else { "▼" };
                let label = format!("{} \u{2261} {}", arrow, name);

                let style = anim
                    .row_style(row_idx, is_selected, focused)
                    .unwrap_or_else(|| {
                        Style::default()
                            .fg(theme::DIM)
                            .bg(theme::BG)
                            .add_modifier(Modifier::BOLD)
                    });

                // Fill background for selected row
                if is_selected {
                    for x in inner.x..inner.x + inner.width {
                        set_cell_if(buf, &bounds, x, y, ' ', style);
                    }
                }

                write_str_clipped(buf, &bounds, inner.x + 1, y, &label, style, &inner);

                // Unread badge for collapsed spaces
                if *collapsed && *unread_count > 0 && !is_selected {
                    let badge = format!("({})", unread_count);
                    let badge_x = inner.x + inner.width - badge.len() as u16 - 1;
                    let badge_style = Style::default().fg(theme::CYAN).bg(theme::BG);
                    write_str_clipped(buf, &bounds, badge_x, y, &badge, badge_style, &inner);
                }
            }
            DisplayRow::Room { room_index, indent } => {
                if let Some(room) = app.room_list.rooms.get(*room_index) {
                    let prefix = match room.category {
                        RoomCategory::Space => "\u{2261} ",
                        RoomCategory::Room => "# ",
                        RoomCategory::DirectMessage => "@ ",
                    };
                    let label = format!("{}{}", prefix, room.name);
                    let indent_px = *indent as u16;

                    let style = anim
                        .row_style(row_idx, is_selected, focused)
                        .unwrap_or_else(theme::text_style);

                    // Fill background for selected row
                    if is_selected {
                        for x in inner.x..inner.x + inner.width {
                            set_cell_if(buf, &bounds, x, y, ' ', style);
                        }
                    }

                    write_str_clipped(
                        buf,
                        &bounds,
                        inner.x + indent_px + 1,
                        y,
                        &label,
                        style,
                        &inner,
                    );

                    // Unread badge
                    if room.unread_count > 0 && !is_selected {
                        let badge = format!("({})", room.unread_count);
                        let badge_x = inner.x + inner.width - badge.len() as u16 - 1;
                        let badge_style = Style::default().fg(theme::CYAN).bg(theme::BG);
                        write_str_clipped(buf, &bounds, badge_x, y, &badge, badge_style, &inner);
                    }
                }
            }
        }
    }
}

#[inline]
fn set_cell_if(buf: &mut Buffer, bounds: &Rect, x: u16, y: u16, ch: char, style: Style) {
    if x >= bounds.x && x < bounds.x + bounds.width && y >= bounds.y && y < bounds.y + bounds.height
    {
        buf[(x, y)].set_char(ch);
        buf[(x, y)].set_style(style);
    }
}

fn write_str_clipped(
    buf: &mut Buffer,
    bounds: &Rect,
    x: u16,
    y: u16,
    text: &str,
    style: Style,
    clip: &Rect,
) {
    for (i, ch) in text.chars().enumerate() {
        let cx = x + i as u16;
        if cx >= clip.x + clip.width {
            break;
        }
        if cx >= bounds.x
            && cx < bounds.x + bounds.width
            && y >= bounds.y
            && y < bounds.y + bounds.height
        {
            buf[(cx, y)].set_char(ch);
            buf[(cx, y)].set_style(style);
        }
    }
}
