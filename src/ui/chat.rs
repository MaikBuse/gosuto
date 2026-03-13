use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};
use ratatui_image::ResizeEncodeRender;

use crate::app::App;
use crate::input::FocusPanel;
use crate::state::{AuthState, MessageContent};
use crate::ui::{gradient, panel, rich_text, theme};

enum ChatSegment<'a> {
    DateSeparator(Line<'a>),
    TextMessage {
        lines: Vec<Line<'a>>,
        msg_index: usize,
    },
    ImageMessage {
        header: Line<'a>,
        event_id: &'a str,
        image_rows: u16,
        loaded: bool,
        failed: bool,
        msg_index: usize,
    },
}

impl ChatSegment<'_> {
    fn msg_index(&self) -> Option<usize> {
        match self {
            ChatSegment::DateSeparator(_) => None,
            ChatSegment::TextMessage { msg_index, .. }
            | ChatSegment::ImageMessage { msg_index, .. } => Some(*msg_index),
        }
    }

    fn height(&self, inner_width: usize) -> usize {
        match self {
            ChatSegment::DateSeparator(_) => 1,
            ChatSegment::TextMessage { lines, .. } => {
                if inner_width > 0 {
                    lines
                        .iter()
                        .map(|line| {
                            let w = line.width();
                            if w == 0 { 1 } else { w.div_ceil(inner_width) }
                        })
                        .sum()
                } else {
                    lines.len()
                }
            }
            ChatSegment::ImageMessage { image_rows, .. } => 1 + *image_rows as usize,
        }
    }
}

fn compute_image_rows(width: Option<u32>, height: Option<u32>, max_cols: u16) -> u16 {
    match (width, height) {
        (Some(w), Some(h)) if w > 0 && h > 0 => {
            let display_w = (w as u16).min(max_cols);
            let aspect = h as f64 / w as f64;
            // Terminal cells are ~2:1 aspect ratio, so halve the row count
            let rows = (display_w as f64 * aspect / 2.0).round() as u16;
            rows.clamp(3, 15)
        }
        _ => 8,
    }
}

pub fn render(app: &mut App, frame: &mut Frame, area: Rect) {
    let focused = app.vim.focus == FocusPanel::Messages;

    let room_name = app
        .messages
        .current_room_id
        .as_ref()
        .and_then(|id| app.room_list.rooms.iter().find(|r| r.id == *id))
        .map(|r| r.name.clone())
        .unwrap_or_else(|| "No room selected".to_string());

    let title_text = format!(" > {} ", room_name);
    let title_line = if focused {
        let revealed = app.chat_title_reveal.revealed_text(&title_text);
        gradient::gradient_title_line(&revealed)
    } else {
        app.chat_title_reveal
            .render_line(&title_text, theme::title_style())
    };

    let block = panel::block_with_bg(title_line, focused, theme::CHAT_BG);

    let inner_height = area.height.saturating_sub(2) as usize; // borders
    let inner_width = area.width.saturating_sub(2) as usize; // borders

    let messages = &app.messages.messages;
    if messages.is_empty() {
        let placeholder = if app.messages.current_room_id.is_none() {
            Paragraph::new(Line::from(Span::styled(
                "Select a room to start chatting",
                theme::dim_style(),
            )))
        } else if app.messages.loading {
            Paragraph::new(Line::from(Span::styled(
                "Loading messages...",
                theme::dim_style(),
            )))
        } else if let Some(ref err) = app.messages.fetch_error {
            Paragraph::new(Line::from(Span::styled(
                format!("Error: {}", err),
                theme::error_style(),
            )))
        } else {
            Paragraph::new(Line::from(Span::styled(
                "No messages yet",
                theme::dim_style(),
            )))
        };
        frame.render_widget(placeholder.block(block), area);
        if focused {
            panel::apply_gradient_border_with_bg(
                frame.buffer_mut(),
                area,
                theme::GRADIENT_BORDER_START,
                theme::GRADIENT_BORDER_END,
                app.anim_clock.phase,
                theme::CHAT_BG,
            );
        }
        return;
    }

    // Build segments
    let max_img_cols = (inner_width.saturating_sub(7) as u16).min(40);
    let mut segments: Vec<ChatSegment> = Vec::new();
    let mut last_date: Option<chrono::NaiveDate> = None;

    for (idx, msg) in messages.iter().enumerate() {
        let msg_date = msg.timestamp.date_naive();
        if last_date != Some(msg_date) {
            let date_str = msg.timestamp.format("%B %-d, %Y").to_string();
            let prefix = "─── ";
            let suffix = " ───";
            let full = format!("{}{}{}", prefix, date_str, suffix);
            let chars: Vec<char> = full.chars().collect();
            let total = chars.len();
            let mid = total as f32 / 2.0;
            let spans: Vec<Span> = chars
                .into_iter()
                .enumerate()
                .map(|(i, ch)| {
                    let dist = ((i as f32) - mid).abs() / mid.max(1.0);
                    let color = gradient::lerp_color(
                        theme::GRADIENT_DATE_BRIGHT,
                        theme::GRADIENT_DATE_DIM,
                        dist,
                    );
                    Span::styled(ch.to_string(), Style::default().fg(color))
                })
                .collect();
            segments.push(ChatSegment::DateSeparator(Line::from(spans)));
            last_date = Some(msg_date);
        }

        let time = msg.timestamp.format("%H:%M").to_string();
        let sender_color = theme::sender_color(&msg.sender);

        let mut spans = gradient::gradient_spans(
            &format!("{} ", time),
            theme::DIM,
            theme::TIMESTAMP_BRIGHT,
            false,
        );

        if msg.verified == Some(false) {
            let icons = app.config.icons();
            spans.push(Span::styled(
                icons.unverified,
                Style::default().fg(theme::RED),
            ));
            spans.push(Span::styled(" ", Style::default().fg(theme::RED)));
        }

        spans.push(Span::styled(
            format!("{} ", msg.sender),
            Style::default()
                .fg(sender_color)
                .add_modifier(Modifier::BOLD),
        ));

        if msg.redacted {
            spans.push(Span::styled("[message deleted]", theme::dim_italic_style()));
            segments.push(ChatSegment::TextMessage {
                lines: vec![Line::from(spans)],
                msg_index: idx,
            });
            continue;
        }

        match &msg.content {
            MessageContent::Text {
                plain: body,
                formatted_html,
            } => {
                let body_style = if msg.pending {
                    theme::dim_style()
                } else if msg.is_emote {
                    Style::default().fg(sender_color)
                } else if msg.is_notice {
                    theme::dim_style()
                } else {
                    theme::text_style()
                };

                let indent_width = 6 + msg.sender.len() + 1;

                let use_rich =
                    formatted_html.is_some() && !msg.pending && !msg.is_emote && !msg.is_notice;

                let mut lines = if use_rich {
                    let html = formatted_html.as_ref().unwrap();
                    let mut rich_lines = rich_text::html_to_lines(html, body_style, indent_width);
                    if rich_lines.is_empty() {
                        if msg.edited {
                            spans.push(Span::styled(" (edited)", theme::dim_style()));
                        }
                        vec![Line::from(spans)]
                    } else {
                        let first = rich_lines.remove(0);
                        spans.extend(first.spans);
                        if msg.edited {
                            spans.push(Span::styled(" (edited)", theme::dim_style()));
                        }
                        let mut result = vec![Line::from(spans)];
                        result.extend(rich_lines);
                        result
                    }
                } else {
                    let body_lines: Vec<&str> = body.split('\n').collect();
                    // First line: attach to the prefix spans
                    if let Some(first) = body_lines.first() {
                        spans.push(Span::styled(first.to_string(), body_style));
                        if msg.pending {
                            spans.push(Span::styled(" (sending...)", theme::dim_style()));
                        } else if msg.edited {
                            spans.push(Span::styled(" (edited)", theme::dim_style()));
                        }
                    }
                    let mut lines = vec![Line::from(spans)];

                    // Continuation lines: indent to align with body text
                    let indent: String = " ".repeat(indent_width);
                    for cont_line in body_lines.iter().skip(1) {
                        lines.push(Line::from(vec![
                            Span::raw(indent.clone()),
                            Span::styled(cont_line.to_string(), body_style),
                        ]));
                    }
                    lines
                };

                // Prepend reply quote line if this message is a reply
                if let Some(ref reply) = msg.in_reply_to {
                    let reply_line = if reply.sender.is_empty() {
                        Line::from(vec![
                            Span::raw("     "),
                            Span::styled("| ", theme::reply_indicator_style()),
                            Span::styled("[unknown message]", theme::dim_style()),
                        ])
                    } else {
                        Line::from(vec![
                            Span::raw("     "),
                            Span::styled("| ", theme::reply_indicator_style()),
                            Span::styled(
                                format!("{}: ", reply.sender),
                                Style::default().fg(theme::sender_color(&reply.sender)),
                            ),
                            Span::styled(reply.body_preview.clone(), theme::dim_style()),
                        ])
                    };
                    lines.insert(0, reply_line);
                }

                if !msg.reactions.is_empty() {
                    let own_id = match &app.auth {
                        AuthState::LoggedIn { user_id, .. } => user_id.as_str(),
                        _ => "",
                    };
                    let mut reaction_spans = vec![Span::raw("      ")];
                    for reaction in &msg.reactions {
                        let is_own = reaction.senders.iter().any(|s| s.user_id == own_id);
                        let badge = format!(" {} {} ", reaction.key, reaction.senders.len());
                        let style = if is_own {
                            theme::reaction_own_badge_style()
                        } else {
                            theme::reaction_badge_style()
                        };
                        reaction_spans.push(Span::styled(badge, style));
                        reaction_spans.push(Span::raw(" "));
                    }
                    lines.push(Line::from(reaction_spans));
                }

                segments.push(ChatSegment::TextMessage {
                    lines,
                    msg_index: idx,
                });
            }
            MessageContent::Image {
                body,
                width,
                height,
            } => {
                spans.push(Span::styled(
                    format!("[image: {}]", body),
                    theme::dim_style(),
                ));
                let header = Line::from(spans);
                let image_rows = compute_image_rows(*width, *height, max_img_cols);
                let loaded = app.image_cache.is_loaded(&msg.event_id);
                let failed = app.image_cache.is_failed(&msg.event_id);
                segments.push(ChatSegment::ImageMessage {
                    header,
                    event_id: &msg.event_id,
                    image_rows,
                    loaded,
                    failed,
                    msg_index: idx,
                });
            }
        }
    }

    // Compute total visual height
    let total_visual_lines: usize = segments.iter().map(|s| s.height(inner_width)).sum();

    // Auto-scroll to keep selected message visible
    let selected_idx = app.messages.selected_index;
    if let Some(sel) = selected_idx {
        let mut cumulative = 0usize;
        let mut sel_start = 0usize;
        let mut sel_end = 0usize;
        for segment in &segments {
            let h = segment.height(inner_width);
            if segment.msg_index() == Some(sel) {
                sel_start = cumulative;
                sel_end = cumulative + h;
                break;
            }
            cumulative += h;
        }
        let max_scroll = total_visual_lines.saturating_sub(inner_height);
        // Convert selection line range to scroll_offset space
        // scroll_y = max_scroll - clamped_offset, so clamped_offset = max_scroll - scroll_y
        // We want sel_start >= scroll_y and sel_end <= scroll_y + inner_height
        let current_offset = app.messages.scroll_offset.min(max_scroll);
        let current_scroll_y = max_scroll.saturating_sub(current_offset);
        let viewport_end = current_scroll_y + inner_height;

        if sel_start < current_scroll_y {
            // Selected is above viewport — scroll up to show it
            let new_scroll_y = sel_start;
            app.messages.scroll_offset = max_scroll.saturating_sub(new_scroll_y);
        } else if sel_end > viewport_end {
            // Selected is below viewport — scroll down to show it
            let new_scroll_y = sel_end.saturating_sub(inner_height);
            app.messages.scroll_offset = max_scroll.saturating_sub(new_scroll_y);
        }
    }

    let max_scroll = total_visual_lines.saturating_sub(inner_height);
    let clamped_offset = app.messages.scroll_offset.min(max_scroll);
    let scroll_y = max_scroll.saturating_sub(clamped_offset);

    // Render the block border first
    frame.render_widget(block, area);

    if focused {
        panel::apply_gradient_border_with_bg(
            frame.buffer_mut(),
            area,
            theme::GRADIENT_BORDER_START,
            theme::GRADIENT_BORDER_END,
            app.anim_clock.phase,
            theme::CHAT_BG,
        );
    }

    // Inner area (inside borders)
    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    // Walk segments and render visible ones
    let mut y_offset: usize = 0; // cumulative visual lines from top
    let viewport_start = scroll_y;
    let viewport_end = scroll_y + inner_height;

    for segment in &segments {
        let seg_height = segment.height(inner_width);
        let seg_start = y_offset;
        let seg_end = y_offset + seg_height;

        y_offset += seg_height;

        // Skip segments entirely above viewport
        if seg_end <= viewport_start {
            continue;
        }
        // Stop if entirely below viewport
        if seg_start >= viewport_end {
            break;
        }

        // Compute the sub-rect for this segment within the viewport
        let render_y = if seg_start >= viewport_start {
            (seg_start - viewport_start) as u16
        } else {
            0
        };
        let clip_top = if seg_start < viewport_start {
            (viewport_start - seg_start) as u16
        } else {
            0
        };
        let available_height = (inner.height - render_y).min(seg_height as u16 - clip_top);

        if available_height == 0 {
            continue;
        }

        let sub_rect = Rect {
            x: inner.x,
            y: inner.y + render_y,
            width: inner.width,
            height: available_height,
        };

        let is_selected = selected_idx.is_some() && segment.msg_index() == selected_idx;

        match segment {
            ChatSegment::DateSeparator(line) => {
                let p = Paragraph::new(line.clone());
                frame.render_widget(p, sub_rect);
            }
            ChatSegment::TextMessage { lines, .. } => {
                let mut p = Paragraph::new(lines.clone())
                    .wrap(Wrap { trim: false })
                    .scroll((clip_top, 0));
                if is_selected {
                    p = p.style(theme::message_selected_style());
                }
                frame.render_widget(p, sub_rect);
            }
            ChatSegment::ImageMessage {
                header,
                event_id,
                image_rows,
                loaded,
                failed,
                ..
            } => {
                // Render header line
                if clip_top == 0 {
                    let header_rect = Rect {
                        height: 1,
                        ..sub_rect
                    };
                    let mut p = Paragraph::new(header.clone());
                    if is_selected {
                        p = p.style(theme::message_selected_style());
                    }
                    frame.render_widget(p, header_rect);
                }

                // Render image area below header
                let img_y_in_seg = 1u16; // image starts at row 1 within segment
                if clip_top < img_y_in_seg + *image_rows {
                    let img_clip = clip_top.saturating_sub(img_y_in_seg);
                    let img_render_y = if clip_top <= img_y_in_seg {
                        sub_rect.y + (img_y_in_seg - clip_top)
                    } else {
                        sub_rect.y
                    };
                    let img_available = (sub_rect.y + sub_rect.height).saturating_sub(img_render_y);
                    let img_height = (*image_rows - img_clip).min(img_available);

                    if img_height > 0 {
                        let default_w = max_img_cols.min(sub_rect.width.saturating_sub(6));
                        let img_w = if let Some(cached) = app.image_cache.get_mut(event_id) {
                            if let (Some(w), Some(h)) = (cached.width, cached.height) {
                                // Each row = 2 pixels (halfblocks)
                                let pixel_h = img_height as f64 * 2.0;
                                let aspect = w as f64 / h as f64;
                                let cols = (pixel_h * aspect).round() as u16;
                                cols.min(default_w)
                            } else {
                                default_w
                            }
                        } else {
                            default_w
                        };

                        let img_rect = Rect {
                            x: sub_rect.x + 6, // indent past timestamp
                            y: img_render_y,
                            width: img_w,
                            height: img_height,
                        };

                        if *loaded {
                            if let Some(cached) = app.image_cache.get_mut(event_id)
                                && let Some(ref mut protocol) = cached.protocol
                            {
                                if cached.last_encoded_rect != Some(img_rect) {
                                    let resize = ratatui_image::Resize::Fit(None);
                                    protocol.resize_encode(&resize, img_rect);
                                    cached.last_encoded_rect = Some(img_rect);
                                }
                                protocol.render(img_rect, frame.buffer_mut());
                            }
                        } else if *failed {
                            let placeholder = Paragraph::new(Line::from(Span::styled(
                                "[failed to load image]",
                                theme::error_style(),
                            )));
                            frame.render_widget(placeholder, img_rect);
                        } else {
                            let placeholder = Paragraph::new(Line::from(Span::styled(
                                "[loading image...]",
                                theme::dim_style(),
                            )));
                            frame.render_widget(placeholder, img_rect);
                        }
                    }
                }
            }
        }
    }

    // Message rain effect: capture snapshot when entering a room
    if app.messages.needs_rain_capture
        && app.effects.enabled
        && !app.effects.message_rain().is_active()
    {
        // Clone the inner chat area as a snapshot for the rain effect
        let mut snapshot = Buffer::empty(inner);
        for y in inner.y..inner.y + inner.height {
            for x in inner.x..inner.x + inner.width {
                snapshot[(x, y)] = frame.buffer_mut()[(x, y)].clone();
            }
        }
        app.effects.message_rain_mut().start(&snapshot, inner);
        app.messages.needs_rain_capture = false;
    }

    // Partial rain for new incoming messages
    if app.messages.rain_pending_count > 0
        && app.effects.enabled
        && !app.effects.message_rain().is_active()
    {
        let mut msgs_remaining = app.messages.rain_pending_count;
        let mut rain_rows: usize = 0;
        for seg in segments.iter().rev() {
            if msgs_remaining == 0 {
                break;
            }
            rain_rows += seg.height(inner_width);
            if seg.msg_index().is_some() {
                msgs_remaining -= 1;
            }
        }

        if rain_rows > 0 {
            // Use the full inner area so cells fall visibly from the top
            let mut snapshot = Buffer::empty(inner);
            // Only copy new message rows (bottom portion) into the snapshot
            let msg_top_y = inner.y + inner.height - (rain_rows as u16).min(inner.height);
            for y in msg_top_y..inner.y + inner.height {
                for x in inner.x..inner.x + inner.width {
                    snapshot[(x, y)] = frame.buffer_mut()[(x, y)].clone();
                }
            }
            app.effects.message_rain_mut().start(&snapshot, inner);
            let clear_rect = Rect {
                x: inner.x,
                y: msg_top_y,
                width: inner.width,
                height: inner.height - (msg_top_y - inner.y),
            };
            app.effects.message_rain_mut().set_clear_rect(clear_rect);
        }
        app.messages.rain_pending_count = 0;
    }

    // Render the message rain animation over the chat area
    if app.effects.message_rain().is_active() {
        app.effects.message_rain_mut().render(frame.buffer_mut());
    }
}
