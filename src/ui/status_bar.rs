use std::sync::atomic::Ordering;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
};

use crate::app::App;
use crate::input::VimMode;
use crate::ui::{gradient, theme};
use crate::voip::CallState;

/// A section of the status bar with text, fg, and bg color.
struct Section {
    text: String,
    fg: Color,
    bg: Color,
    bold: bool,
}

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let icons = app.config.icons();
    let powerline = icons.powerline_right;

    let (mode_fg, mode_bg) = match app.vim.mode {
        VimMode::Normal => (theme::BLACK, theme::CYAN),
        VimMode::Insert => (theme::BLACK, theme::GREEN),
        VimMode::Command => (theme::BLACK, theme::MAGENTA),
    };

    let mode_bg_dim = gradient::scale_color(mode_bg, 0.4);

    let mode_label = format!(" {} ", app.vim.mode);

    let room_name = app
        .room_list
        .selected_room()
        .map(|r| format!(" {} ", r.name))
        .unwrap_or_default();

    let sync_status = format!(" {} ", app.sync_status);

    // Build sections
    let mut sections: Vec<Section> = Vec::new();

    // Mode indicator with gradient bg
    sections.push(Section {
        text: mode_label,
        fg: mode_fg,
        bg: mode_bg,
        bold: true,
    });

    // Room name
    if !room_name.is_empty() {
        sections.push(Section {
            text: room_name,
            fg: theme::TEXT,
            bg: theme::STATUS_BAR_BG,
            bold: false,
        });
    }

    // Sync status
    sections.push(Section {
        text: sync_status,
        fg: theme::DIM,
        bg: theme::STATUS_BAR_BG,
        bold: false,
    });

    // Call status
    let participants_label = |info: &crate::voip::CallInfo| -> String {
        if info.participants.is_empty() {
            String::new()
        } else if info.participants.len() == 1 {
            info.participants[0].clone()
        } else {
            format!("{} participants", info.participants.len())
        }
    };

    if let Some(ref info) = app.call_info {
        let label = participants_label(info);
        match info.state {
            CallState::Connecting => {
                sections.push(Section {
                    text: format!(" CONNECTING: {} ", label),
                    fg: theme::CYAN,
                    bg: theme::STATUS_BAR_BG,
                    bold: true,
                });
            }
            CallState::Active => {
                sections.push(Section {
                    text: format!(" CALL {} {} ", info.elapsed_display(), label),
                    fg: theme::GREEN,
                    bg: theme::STATUS_BAR_BG,
                    bold: true,
                });
            }
        }
    } else if let Some(ref caller) = app.incoming_call_user {
        sections.push(Section {
            text: format!(" INCOMING: {} ", caller),
            fg: theme::GREEN,
            bg: theme::STATUS_BAR_BG,
            bold: true,
        });
    }

    // Mic status
    if let Some(ref info) = app.call_info
        && matches!(info.state, CallState::Active)
    {
        if app.mic_active.load(Ordering::Relaxed) {
            sections.push(Section {
                text: " \u{25cf} MIC".to_string(),
                fg: theme::GREEN,
                bg: theme::STATUS_BAR_BG,
                bold: true,
            });
        } else {
            sections.push(Section {
                text: " \u{25cb} MIC".to_string(),
                fg: theme::DIM,
                bg: theme::STATUS_BAR_BG,
                bold: false,
            });
        }
    }

    // Verify status
    if let Some(ref modal) = app.verification_modal {
        sections.push(Section {
            text: format!(" VERIFYING: {} ", modal.sender),
            fg: theme::CYAN,
            bg: theme::STATUS_BAR_BG,
            bold: true,
        });
    }

    // Error
    if let Some(ref err) = app.last_error {
        sections.push(Section {
            text: format!(" {} ", err),
            fg: theme::RED,
            bg: theme::STATUS_BAR_BG,
            bold: false,
        });
    }

    // Direct buffer writes
    let buf = frame.buffer_mut();
    let bounds = *buf.area();

    // Fill entire bar with STATUS_BAR_BG
    let bar_style = Style::default().bg(theme::STATUS_BAR_BG);
    for x in area.x..area.x + area.width {
        if x < bounds.x + bounds.width && area.y < bounds.y + bounds.height {
            let cell = &mut buf[(x, area.y)];
            cell.set_char(' ');
            cell.set_style(bar_style);
        }
    }

    let mut cursor_x = area.x;

    for (i, section) in sections.iter().enumerate() {
        if section.text.is_empty() {
            continue;
        }

        // Write gradient mode indicator for first section
        let section_bg = if i == 0 {
            // Gradient bg across mode label width
            let chars: Vec<char> = section.text.chars().collect();
            for (ci, ch) in chars.iter().enumerate() {
                if cursor_x + ci as u16 >= area.x + area.width {
                    break;
                }
                let x = cursor_x + ci as u16;
                if x < bounds.x + bounds.width && area.y < bounds.y + bounds.height {
                    let t = ci as f32 / chars.len().max(1) as f32;
                    let bg = gradient::lerp_color(section.bg, mode_bg_dim, t);
                    let mut style = Style::default().fg(section.fg).bg(bg);
                    if section.bold {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    let cell = &mut buf[(x, area.y)];
                    cell.set_char(*ch);
                    cell.set_style(style);
                }
            }
            cursor_x += chars.len() as u16;
            mode_bg_dim
        } else {
            // Regular section text
            let mut style = Style::default().fg(section.fg).bg(section.bg);
            if section.bold {
                style = style.add_modifier(Modifier::BOLD);
            }
            for ch in section.text.chars() {
                if cursor_x >= area.x + area.width {
                    break;
                }
                if cursor_x < bounds.x + bounds.width && area.y < bounds.y + bounds.height {
                    let cell = &mut buf[(cursor_x, area.y)];
                    cell.set_char(ch);
                    cell.set_style(style);
                }
                cursor_x += 1;
            }
            section.bg
        };

        // Powerline separator between sections
        let next_bg = sections
            .get(i + 1)
            .map(|s| s.bg)
            .unwrap_or(theme::STATUS_BAR_BG);

        if cursor_x < area.x + area.width
            && cursor_x < bounds.x + bounds.width
            && area.y < bounds.y + bounds.height
        {
            let sep_style = Style::default().fg(section_bg).bg(next_bg);
            let cell = &mut buf[(cursor_x, area.y)];
            cell.set_char(powerline.chars().next().unwrap_or('▸'));
            cell.set_style(sep_style);
            cursor_x += 1;
        }
    }
}
