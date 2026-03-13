use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};

use crate::app::App;
use crate::input::FocusPanel;
use crate::state::{DisplayRow, RoomCategory};
use crate::ui::tooltip::{self, Direction, set_cell_if, write_str_clipped};
use crate::ui::{gradient, panel, theme};

pub struct RoomListAnimState {
    pub flash_timer: Option<u64>,
    pub flash_row: Option<usize>,
}

impl RoomListAnimState {
    pub fn new() -> Self {
        Self {
            flash_timer: None,
            flash_row: None,
        }
    }

    pub fn tick(&mut self, dt_ms: u64) {
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

    fn pulse_style(&self, phase: f32) -> Style {
        let t = (phase.sin() + 1.0) / 2.0;
        let shimmer = ((phase * 3.7).sin() + 1.0) / 2.0;
        let factor = 0.45 + (t + shimmer * 0.08) * 0.55;

        Style::default()
            .fg(theme::BLACK)
            .bg(gradient::scale_color(theme::PULSE_BASE, factor))
            .add_modifier(Modifier::BOLD)
    }

    fn flash_style(&self) -> Option<Style> {
        let remaining = self.flash_timer?;
        let intensity = (remaining as f32 / 250.0).powi(2);
        let inv = 1.0 - intensity;

        Some(
            Style::default()
                .fg(gradient::lerp_color(theme::BLACK, theme::WHITE, inv))
                .bg(gradient::lerp_color(theme::CYAN, theme::WHITE, intensity))
                .add_modifier(Modifier::BOLD),
        )
    }

    fn row_style(
        &self,
        row_idx: usize,
        is_selected: bool,
        focused: bool,
        phase: f32,
    ) -> Option<Style> {
        if !is_selected {
            return None;
        }
        if focused {
            if self.flash_row == Some(row_idx)
                && let Some(flash) = self.flash_style()
            {
                return Some(flash);
            }
            Some(self.pulse_style(phase))
        } else {
            // Static cyan highlight when panel is not focused
            Some(
                Style::default()
                    .fg(theme::BLACK)
                    .bg(theme::CYAN)
                    .add_modifier(Modifier::BOLD),
            )
        }
    }
}

/// Compute the scroll offset for the room list given the current state and area.
pub fn scroll_offset(app: &App, area: Rect) -> usize {
    let inner_height = area.height.saturating_sub(2) as usize;
    panel::scroll_offset(
        app.room_list.display_rows.len(),
        app.room_list.selected,
        inner_height,
    )
}

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.vim.focus == FocusPanel::RoomList;
    let icons = app.config.icons();

    let title_line = if focused {
        gradient::gradient_title_line(" ROOMS ")
    } else {
        Line::from(vec![Span::styled(" ROOMS ", theme::title_style())])
    };
    let block = panel::block_with_bg(title_line, focused, theme::SIDEBAR_BG);

    frame.render_widget(block, area);

    if focused {
        panel::apply_gradient_border_with_bg(
            frame.buffer_mut(),
            area,
            theme::GRADIENT_BORDER_START,
            theme::GRADIENT_BORDER_END,
            app.anim_clock.phase,
            theme::SIDEBAR_BG,
        );
    }

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

    if app.room_list.loading && app.room_list.display_rows.is_empty() {
        let phase = app.anim_clock.phase;
        let dots = match ((phase / std::f32::consts::TAU * 3.0) as usize) % 3 {
            0 => "·  ",
            1 => "·· ",
            _ => "···",
        };
        let text = format!("Loading rooms{dots}");
        let style = theme::dim_style();
        let text_width = text.chars().count() as u16;
        let x = inner.x + inner.width.saturating_sub(text_width) / 2;
        let y = inner.y + inner.height / 2;
        let buf = frame.buffer_mut();
        write_str_clipped(buf, x, y, &text, style, &inner, false);
        return;
    }

    let display_rows = &app.room_list.display_rows;
    let selected = app.room_list.selected;
    let anim = &app.room_list_anim;
    let phase = app.anim_clock.phase;
    let visible_height = inner.height as usize;
    let total_rows = display_rows.len();

    let scroll_offset = panel::scroll_offset(total_rows, selected, visible_height);

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
                    .bg(theme::SIDEBAR_BG)
                    .add_modifier(Modifier::BOLD);
                let text = format!(" {} ", label);
                write_str_clipped(buf, inner.x + 1, y, &text, style, &inner, true);
                // Fill remaining with ─
                let line_style = Style::default().fg(theme::DIM).bg(theme::SIDEBAR_BG);
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
                let arrow = if *collapsed {
                    icons.collapse
                } else {
                    icons.expand
                };
                let label = format!("{} {} {}", arrow, icons.space, name);

                let style = anim
                    .row_style(row_idx, is_selected, focused, phase)
                    .unwrap_or_else(|| {
                        Style::default()
                            .fg(theme::DIM)
                            .bg(theme::SIDEBAR_BG)
                            .add_modifier(Modifier::BOLD)
                    });

                // Fill background for selected row
                if is_selected && focused {
                    gradient::fill_row_highlight(buf, bounds, y, inner.x, inner.width, true);
                } else if is_selected {
                    for x in inner.x..inner.x + inner.width {
                        set_cell_if(buf, &bounds, x, y, ' ', style);
                    }
                }

                // Narrow clip rect to reserve space for unread badge
                let name_clip = if *collapsed && *unread_count > 0 && !is_selected {
                    let badge_len = format!("({})", unread_count).len() as u16 + 2;
                    Rect::new(
                        inner.x,
                        inner.y,
                        inner.width.saturating_sub(badge_len),
                        inner.height,
                    )
                } else {
                    inner
                };
                write_str_clipped(buf, inner.x + 1, y, &label, style, &name_clip, true);

                // Unread badge for collapsed spaces
                if *collapsed && *unread_count > 0 && !is_selected {
                    let badge = format!("({})", unread_count);
                    let badge_x = inner.x + inner.width - badge.len() as u16 - 1;
                    let badge_style = Style::default().fg(theme::CYAN).bg(theme::UNREAD_BADGE_BG);
                    write_str_clipped(buf, badge_x, y, &badge, badge_style, &inner, false);
                }
            }
            DisplayRow::Room { room_index, indent } => {
                if let Some(room) = app.room_list.rooms.get(*room_index) {
                    let prefix = match room.category {
                        RoomCategory::Invitation => icons.invite,
                        RoomCategory::Space => icons.space,
                        RoomCategory::Room => icons.room,
                        RoomCategory::DirectMessage => icons.dm,
                    };
                    let label = format!("{} {}", prefix, room.name);
                    let indent_px = *indent as u16;

                    let style = anim
                        .row_style(row_idx, is_selected, focused, phase)
                        .unwrap_or_else(theme::text_style);

                    // Fill background for selected row
                    if is_selected && focused {
                        gradient::fill_row_highlight(buf, bounds, y, inner.x, inner.width, true);
                    } else if is_selected {
                        for x in inner.x..inner.x + inner.width {
                            set_cell_if(buf, &bounds, x, y, ' ', style);
                        }
                    }

                    // Narrow clip rect to reserve space for unread badge
                    let name_clip = if room.unread_count > 0 && !is_selected {
                        let badge_len = format!("({})", room.unread_count).len() as u16 + 2;
                        Rect::new(
                            inner.x,
                            inner.y,
                            inner.width.saturating_sub(badge_len),
                            inner.height,
                        )
                    } else {
                        inner
                    };
                    write_str_clipped(
                        buf,
                        inner.x + indent_px + 1,
                        y,
                        &label,
                        style,
                        &name_clip,
                        true,
                    );

                    // Unread badge
                    if room.unread_count > 0 && !is_selected {
                        let badge = format!("({})", room.unread_count);
                        let badge_x = inner.x + inner.width - badge.len() as u16 - 1;
                        let badge_style =
                            Style::default().fg(theme::CYAN).bg(theme::UNREAD_BADGE_BG);
                        write_str_clipped(buf, badge_x, y, &badge, badge_style, &inner, false);
                    }
                }
            }
            DisplayRow::CallParticipant { display_name } => {
                let label = format!("    > {}", display_name);
                let style = Style::default().fg(theme::GREEN).bg(theme::SIDEBAR_BG);
                write_str_clipped(buf, inner.x + 1, y, &label, style, &inner, true);
            }
        }
    }
}

/// Render a floating tooltip showing the full room name when the selected row is truncated.
pub fn render_tooltip(app: &App, frame: &mut Frame, room_list_area: Rect) {
    // Only show tooltip when room list is focused
    if app.vim.focus != FocusPanel::RoomList {
        return;
    }

    let display_rows = &app.room_list.display_rows;
    let selected = app.room_list.selected;

    let row = match display_rows.get(selected) {
        Some(r) => r,
        None => return,
    };

    let icons = app.config.icons();

    // Reconstruct the full label (same format as render())
    let label = match row {
        DisplayRow::Room {
            room_index,
            indent: _,
        } => {
            let room = match app.room_list.rooms.get(*room_index) {
                Some(r) => r,
                None => return,
            };
            let prefix = match room.category {
                RoomCategory::Invitation => icons.invite,
                RoomCategory::Space => icons.space,
                RoomCategory::Room => icons.room,
                RoomCategory::DirectMessage => icons.dm,
            };
            format!("{} {}", prefix, room.name)
        }
        DisplayRow::SpaceHeader {
            name, collapsed, ..
        } => {
            let arrow = if *collapsed {
                icons.collapse
            } else {
                icons.expand
            };
            format!("{} {} {}", arrow, icons.space, name)
        }
        _ => return,
    };

    // Inner area of the room list panel (inside borders)
    let inner_width = room_list_area.width.saturating_sub(2) as usize;
    if inner_width == 0 {
        return;
    }

    // Check if label is truncated (account for 1-char left padding)
    let available_cols = inner_width.saturating_sub(1);
    if label.chars().count() <= available_cols {
        return; // Not truncated, no tooltip needed
    }

    // Compute the selected row's screen y-position
    let inner_height = room_list_area.height.saturating_sub(2) as usize;
    if inner_height == 0 {
        return;
    }

    let scroll_off = panel::scroll_offset(display_rows.len(), selected, inner_height);

    // Check if selected row is visible
    if selected < scroll_off || selected >= scroll_off + inner_height {
        return;
    }

    let row_y = room_list_area.y + 1 + (selected - scroll_off) as u16;

    // Position tooltip to the right of the room list panel
    let anchor_x = room_list_area.x + room_list_area.width;
    let term = frame.area();
    let buf = frame.buffer_mut();

    tooltip::render_tooltip_box(buf, term, &label, anchor_x, row_y, Direction::Right);
}
