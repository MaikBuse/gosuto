use ratatui::{
    Frame,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{List, ListItem, ListState},
};

use crate::app::App;
use crate::input::FocusPanel;
use crate::ui::icons::Icons;
use crate::ui::tooltip::{self, Direction};
use crate::ui::{gradient, panel, theme};

/// IRC-style power level prefix
fn power_prefix(power_level: i64, icons: &Icons) -> &str {
    match power_level {
        100 => icons.power_owner,
        75..=99 => icons.power_admin,
        50..=74 => icons.power_mod,
        1..=49 => icons.power_voice,
        _ => icons.power_none,
    }
}

pub fn scroll_offset(app: &App, area: Rect) -> usize {
    let inner_height = area.height.saturating_sub(2) as usize;
    panel::scroll_offset(
        app.members_list.members.len(),
        app.members_list.selected,
        inner_height,
    )
}

pub fn render(app: &App, frame: &mut Frame, area: Rect) {
    let focused = app.vim.focus == FocusPanel::Members;

    let member_count = app.members_list.members.len();
    let title = format!(" MEMBERS ({}) ", member_count);

    let title_line = if focused {
        let revealed = app.members_title_reveal.revealed_text(&title);
        gradient::gradient_title_line(&revealed)
    } else {
        app.members_title_reveal
            .render_line(&title, theme::title_style())
    };

    let block = panel::block_with_bg(title_line, focused, theme::SIDEBAR_BG);

    let icons = app.config.icons();

    let items: Vec<ListItem> = app
        .members_list
        .members
        .iter()
        .map(|member| {
            let prefix = power_prefix(member.power_level, icons);
            let prefix_style = if member.power_level >= 50 {
                Style::default().fg(theme::GREEN)
            } else if member.power_level > 0 {
                Style::default().fg(theme::CYAN)
            } else {
                theme::dim_style()
            };

            let name_color = theme::sender_color(&member.user_id);
            let name_style = Style::default().fg(name_color);

            let id_label = member.user_id.strip_prefix('@').unwrap_or(&member.user_id);
            let name_with_id = format!("{} ({})", member.display_name, id_label);
            let mut spans = vec![
                Span::styled(prefix, prefix_style),
                Span::raw(" "),
                Span::styled(name_with_id, name_style),
            ];

            if member.verified == Some(true) {
                spans.push(Span::styled(
                    format!(" {}", icons.checkmark),
                    Style::default().fg(theme::GREEN),
                ));
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let highlight_style = if focused {
        theme::highlight_focused_style()
    } else {
        theme::highlight_unfocused_style()
    };

    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style);

    let mut list_state = ListState::default();
    if !app.members_list.members.is_empty() {
        list_state.select(Some(app.members_list.selected));
    }

    frame.render_stateful_widget(list, area, &mut list_state);

    if focused {
        panel::apply_gradient_border_with_bg(
            frame.buffer_mut(),
            area,
            theme::GRADIENT_BORDER_START,
            theme::GRADIENT_BORDER_END,
            app.anim_clock.phase,
            theme::SIDEBAR_BG,
        );

        // Post-pass: gradient highlight on selected row
        if !app.members_list.members.is_empty() {
            let inner = Rect::new(
                area.x + 1,
                area.y + 1,
                area.width.saturating_sub(2),
                area.height.saturating_sub(2),
            );
            let inner_height = inner.height as usize;
            let scroll_off = scroll_offset(app, area);
            let sel = app.members_list.selected;
            if sel >= scroll_off && sel < scroll_off + inner_height {
                let row_y = inner.y + (sel - scroll_off) as u16;
                let buf = frame.buffer_mut();
                let bounds = *buf.area();
                gradient::fill_row_highlight(buf, bounds, row_y, inner.x, inner.width, false);
            }
        }
    }
}

/// Render a floating tooltip showing the full member label when it overflows the pane.
pub fn render_tooltip(app: &App, frame: &mut Frame, members_area: Rect) {
    if app.vim.focus != FocusPanel::Members {
        return;
    }

    let members = &app.members_list.members;
    let selected = app.members_list.selected;

    let member = match members.get(selected) {
        Some(m) => m,
        None => return,
    };

    let icons = app.config.icons();

    // Reconstruct the full label (same format as render())
    let prefix = power_prefix(member.power_level, icons);
    let id_label = member.user_id.strip_prefix('@').unwrap_or(&member.user_id);
    let mut label = format!("{}{} ({})", prefix, member.display_name, id_label);
    if member.verified == Some(true) {
        label.push(' ');
        label.push_str(icons.checkmark);
    }

    // Inner width of the members pane (inside borders)
    let inner_width = members_area.width.saturating_sub(2) as usize;
    if inner_width == 0 {
        return;
    }

    // Check if label overflows (List widget adds 0 padding by default)
    if label.chars().count() <= inner_width {
        return;
    }

    // Compute selected row's screen y-position
    let inner_height = members_area.height.saturating_sub(2) as usize;
    if inner_height == 0 {
        return;
    }

    let scroll_off = scroll_offset(app, members_area);

    if selected < scroll_off || selected >= scroll_off + inner_height {
        return;
    }

    let row_y = members_area.y + 1 + (selected - scroll_off) as u16;

    // Position tooltip to the left of the members pane
    let anchor_x = members_area.x;
    let term = frame.area();
    let buf = frame.buffer_mut();

    tooltip::render_tooltip_box(buf, term, &label, anchor_x, row_y, Direction::Left);
}
