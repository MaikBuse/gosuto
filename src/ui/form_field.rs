use ratatui::buffer::Buffer;
use ratatui::style::Style;

use crate::ui::icons::Icons;
use crate::ui::popup;
use crate::ui::theme;

/// Render the marker icon + label text for a form field row.
pub fn render_label(
    buf: &mut Buffer,
    left: u16,
    label_x: u16,
    row: u16,
    label: &str,
    selected: bool,
    icons: &Icons,
) {
    let bounds = *buf.area();
    let marker = if selected {
        icons.selected
    } else {
        icons.unselected
    };

    popup::write_str(
        buf,
        &bounds,
        left,
        row,
        marker,
        theme::field_marker_style(selected),
    );
    popup::set_cell(
        buf,
        &bounds,
        left + 1,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );
    popup::write_str(
        buf,
        &bounds,
        label_x,
        row,
        label,
        theme::field_label_style(selected),
    );
}

/// Render a truncated text value for a non-editing form field.
pub fn render_value(
    buf: &mut Buffer,
    value_x: u16,
    right: u16,
    row: u16,
    value: &str,
    selected: bool,
) {
    let bounds = *buf.area();
    let max_w = (right - value_x) as usize;
    let display = popup::truncate_str(value, max_w);
    popup::write_str(
        buf,
        &bounds,
        value_x,
        row,
        &display,
        theme::field_value_style(selected),
    );
}

/// Render an editable text buffer with a blinking cursor.
pub fn render_editing(
    buf: &mut Buffer,
    value_x: u16,
    right: u16,
    row: u16,
    text: &str,
    cursor_visible: bool,
) {
    let bounds = *buf.area();
    let max_w = (right - value_x) as usize;
    let display = popup::truncate_str(text, max_w.saturating_sub(1));
    popup::write_str(
        buf,
        &bounds,
        value_x,
        row,
        &display,
        theme::edit_text_style(),
    );
    if cursor_visible {
        let cursor_x = value_x + display.chars().count() as u16;
        popup::set_cell(buf, &bounds, cursor_x, row, '_', theme::edit_cursor_style());
    }
}

/// Render a `< value >` cycle selector.
pub fn render_cycle_selector(
    buf: &mut Buffer,
    value_x: u16,
    row: u16,
    value: &str,
    selected: bool,
    icons: &Icons,
) {
    let bounds = *buf.area();
    let arrow_s = theme::field_arrow_style(selected);
    let val_s = theme::field_value_style(selected);

    popup::write_str(buf, &bounds, value_x, row, icons.arrow_left, arrow_s);
    popup::set_cell(
        buf,
        &bounds,
        value_x + 1,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );

    popup::write_str(buf, &bounds, value_x + 2, row, value, val_s);

    let end_x = value_x + 2 + value.chars().count() as u16;
    popup::set_cell(
        buf,
        &bounds,
        end_x,
        row,
        ' ',
        Style::default().bg(theme::BG),
    );
    popup::write_str(buf, &bounds, end_x + 1, row, icons.arrow_right, arrow_s);
}
