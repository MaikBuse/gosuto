use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
};

use super::theme;

/// Convert Matrix HTML into styled ratatui `Line`s.
///
/// - `base_style`: default text style (varies per message type)
/// - `indent_width`: spaces to prepend on continuation lines (index > 0)
/// - Returns empty `Vec` for empty input.
pub fn html_to_lines(html: &str, base_style: Style, indent_width: usize) -> Vec<Line<'static>> {
    if html.is_empty() {
        return Vec::new();
    }
    let mut parser = Parser::new(base_style, indent_width);
    parser.parse(html);
    parser.finish()
}

#[derive(Clone)]
enum TagKind {
    Bold,
    Italic,
    Strikethrough,
    CodeInline,
    CodeBlock,
    Link,
    Blockquote,
    Heading,
}

#[derive(Clone)]
struct StyleEntry {
    kind: TagKind,
    style: Style,
}

struct Parser {
    base_style: Style,
    indent_width: usize,
    style_stack: Vec<StyleEntry>,
    blockquote_depth: usize,
    list_stack: Vec<Option<usize>>,
    in_mx_reply: bool,
    in_pre: bool,
    heading_level: Option<u8>,
    link_href: Option<String>,
    link_text_start: Option<usize>,
    lines: Vec<Vec<Span<'static>>>,
    current_spans: Vec<Span<'static>>,
    text_buf: String,
}

impl Parser {
    fn new(base_style: Style, indent_width: usize) -> Self {
        Self {
            base_style,
            indent_width,
            style_stack: Vec::new(),
            blockquote_depth: 0,
            list_stack: Vec::new(),
            in_mx_reply: false,
            in_pre: false,
            heading_level: None,
            link_href: None,
            link_text_start: None,
            lines: Vec::new(),
            current_spans: Vec::new(),
            text_buf: String::new(),
        }
    }

    fn current_style(&self) -> Style {
        let mut style = self.base_style;
        for entry in &self.style_stack {
            style = style.patch(entry.style);
        }
        style
    }

    fn flush_text(&mut self) {
        if self.text_buf.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.text_buf);
        let style = self.current_style();
        self.current_spans.push(Span::styled(text, style));
    }

    fn flush_line(&mut self) {
        self.flush_text();
        let mut final_spans: Vec<Span<'static>> = Vec::new();

        let line_index = self.lines.len();
        if line_index > 0 && self.indent_width > 0 {
            final_spans.push(Span::raw(" ".repeat(self.indent_width)));
        }

        for _ in 0..self.blockquote_depth {
            final_spans.push(Span::styled("│ ".to_string(), theme::blockquote_style()));
        }

        final_spans.append(&mut self.current_spans);
        self.lines.push(final_spans);
    }

    fn parse(&mut self, html: &str) {
        let bytes = html.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            match bytes[i] {
                b'<' => {
                    self.flush_text();
                    if let Some((tag, end)) = parse_tag(bytes, i) {
                        if self.in_mx_reply && !(tag.is_close && tag.name == "mx-reply") {
                            i = end;
                            continue;
                        }
                        if tag.is_close {
                            self.handle_close_tag(&tag.name);
                        } else {
                            self.handle_open_tag(&tag.name, &tag.attrs, tag.self_closing);
                        }
                        i = end;
                    } else {
                        // Lone '<' — treat as literal
                        self.append_char('<');
                        i += 1;
                    }
                }
                b'&' => {
                    if let Some((ch, consumed)) = decode_entity(bytes, i) {
                        self.append_char(ch);
                        i += consumed;
                    } else {
                        self.append_char('&');
                        i += 1;
                    }
                }
                _ => {
                    let ch = if bytes[i] < 0x80 {
                        i += 1;
                        bytes[i - 1] as char
                    } else {
                        let s = &html[i..];
                        let c = s.chars().next().unwrap();
                        i += c.len_utf8();
                        c
                    };

                    if self.in_mx_reply {
                        continue;
                    }

                    if self.in_pre {
                        if ch == '\n' {
                            self.flush_line();
                        } else {
                            self.text_buf.push(ch);
                        }
                    } else {
                        // Collapse whitespace
                        if ch.is_whitespace() {
                            // Only add space if we have non-space content before
                            let has_prior = !self.text_buf.is_empty()
                                || !self.current_spans.is_empty()
                                || !self.lines.is_empty();
                            if has_prior && !self.text_buf.ends_with(' ') {
                                self.text_buf.push(' ');
                            }
                        } else {
                            self.text_buf.push(ch);
                        }
                    }
                }
            }
        }
    }

    fn append_char(&mut self, ch: char) {
        if self.in_mx_reply {
            return;
        }
        self.text_buf.push(ch);
    }

    fn handle_open_tag(&mut self, name: &str, attrs: &str, self_closing: bool) {
        match name {
            "b" | "strong" => {
                self.style_stack.push(StyleEntry {
                    kind: TagKind::Bold,
                    style: Style::default().add_modifier(Modifier::BOLD),
                });
            }
            "i" | "em" => {
                self.style_stack.push(StyleEntry {
                    kind: TagKind::Italic,
                    style: Style::default().add_modifier(Modifier::ITALIC),
                });
            }
            "del" | "s" => {
                self.style_stack.push(StyleEntry {
                    kind: TagKind::Strikethrough,
                    style: Style::default().add_modifier(Modifier::CROSSED_OUT),
                });
            }
            "code" => {
                if !self.in_pre {
                    self.style_stack.push(StyleEntry {
                        kind: TagKind::CodeInline,
                        style: theme::code_inline_style(),
                    });
                }
            }
            "pre" => {
                self.flush_line();
                self.in_pre = true;
                self.style_stack.push(StyleEntry {
                    kind: TagKind::CodeBlock,
                    style: theme::code_block_style(),
                });
            }
            "a" => {
                let href = extract_href(attrs).unwrap_or_default();
                self.link_href = Some(href);
                self.flush_text();
                self.link_text_start = Some(self.current_spans.len());
                self.style_stack.push(StyleEntry {
                    kind: TagKind::Link,
                    style: theme::link_style(),
                });
            }
            "blockquote" => {
                self.flush_line();
                self.blockquote_depth += 1;
                self.style_stack.push(StyleEntry {
                    kind: TagKind::Blockquote,
                    style: theme::blockquote_style(),
                });
            }
            "ul" => {
                self.list_stack.push(None);
            }
            "ol" => {
                self.list_stack.push(Some(0));
            }
            "li" => {
                self.flush_line();
                let prefix = match self.list_stack.last_mut() {
                    Some(Some(counter)) => {
                        *counter += 1;
                        format!("{}. ", *counter)
                    }
                    Some(None) => "• ".to_string(),
                    None => "• ".to_string(),
                };
                self.current_spans
                    .push(Span::styled(prefix, self.current_style()));
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.flush_line();
                let level = name.as_bytes()[1] - b'0';
                self.heading_level = Some(level);
                self.style_stack.push(StyleEntry {
                    kind: TagKind::Heading,
                    style: Style::default().add_modifier(Modifier::BOLD),
                });
            }
            "br" => {
                self.flush_line();
            }
            "p" => {
                // Only flush if we have content (avoid blank lines at start)
                if !self.current_spans.is_empty()
                    || !self.text_buf.is_empty()
                    || !self.lines.is_empty()
                {
                    self.flush_line();
                }
            }
            "mx-reply" => {
                self.in_mx_reply = true;
            }
            _ => {} // Unknown tags silently ignored
        }

        // Handle self-closing tags that need close behavior
        if self_closing {
            match name {
                "br" => {} // already handled
                _ => self.handle_close_tag(name),
            }
        }
    }

    fn handle_close_tag(&mut self, name: &str) {
        match name {
            "b" | "strong" => self.pop_style(TagKind::Bold),
            "i" | "em" => self.pop_style(TagKind::Italic),
            "del" | "s" => self.pop_style(TagKind::Strikethrough),
            "code" => {
                if !self.in_pre {
                    self.pop_style(TagKind::CodeInline);
                }
            }
            "pre" => {
                self.flush_line();
                self.in_pre = false;
                self.pop_style(TagKind::CodeBlock);
            }
            "a" => {
                self.flush_text();
                // Check if we should append the URL
                if let (Some(start), Some(href)) =
                    (self.link_text_start.take(), self.link_href.take())
                {
                    // Collect link text from spans
                    let link_text: String = self.current_spans[start..]
                        .iter()
                        .map(|s| s.content.as_ref())
                        .collect();
                    if !href.is_empty() && href != link_text {
                        let url_span = Span::styled(
                            format!(" ({href})"),
                            self.base_style.add_modifier(Modifier::DIM),
                        );
                        self.current_spans.push(url_span);
                    }
                }
                self.pop_style(TagKind::Link);
            }
            "blockquote" => {
                self.flush_line();
                self.blockquote_depth = self.blockquote_depth.saturating_sub(1);
                self.pop_style(TagKind::Blockquote);
            }
            "ul" | "ol" => {
                self.list_stack.pop();
            }
            "li" => {
                // No special close action needed
            }
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                // For h1, uppercase the text in current spans
                if self.heading_level == Some(1) {
                    self.flush_text();
                    for span in &mut self.current_spans {
                        let upper = span.content.to_uppercase();
                        *span = Span::styled(upper, span.style);
                    }
                }
                self.flush_line();
                self.heading_level = None;
                self.pop_style(TagKind::Heading);
            }
            "p" => {
                self.flush_line();
            }
            "mx-reply" => {
                self.in_mx_reply = false;
            }
            _ => {}
        }
    }

    fn pop_style(&mut self, kind: TagKind) {
        let discriminant = std::mem::discriminant(&kind);
        // Find and remove from top of stack for malformed HTML tolerance
        if let Some(pos) = self
            .style_stack
            .iter()
            .rposition(|e| std::mem::discriminant(&e.kind) == discriminant)
        {
            self.flush_text();
            self.style_stack.remove(pos);
        }
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        // Flush remaining text
        if !self.text_buf.is_empty() || !self.current_spans.is_empty() {
            self.flush_line();
        }

        // Trim trailing empty lines
        while self.lines.last().is_some_and(|spans| {
            spans.is_empty() || spans.iter().all(|s| s.content.trim().is_empty())
        }) {
            self.lines.pop();
        }

        self.lines.into_iter().map(Line::from).collect()
    }
}

// --- Tag parsing ---

struct Tag {
    name: String,
    attrs: String,
    is_close: bool,
    self_closing: bool,
}

/// Parse a tag starting at position `start` (which points to `<`).
/// Returns `(Tag, end_position)` where end_position is one past `>`.
fn parse_tag(bytes: &[u8], start: usize) -> Option<(Tag, usize)> {
    let len = bytes.len();
    if start >= len || bytes[start] != b'<' {
        return None;
    }

    // Find closing '>'
    let end = bytes[start..]
        .iter()
        .position(|&b| b == b'>')
        .map(|i| start + i + 1)?;

    let inner = &bytes[start + 1..end - 1];
    if inner.is_empty() {
        return None;
    }

    let inner_str = std::str::from_utf8(inner).ok()?;
    let inner_str = inner_str.trim();

    if inner_str.is_empty() {
        return None;
    }

    let is_close = inner_str.starts_with('/');
    let content = if is_close { &inner_str[1..] } else { inner_str };

    let self_closing = content.ends_with('/');
    let content = if self_closing {
        &content[..content.len() - 1]
    } else {
        content
    };

    // Split name from attrs
    let (name, attrs) = match content.find(|c: char| c.is_whitespace()) {
        Some(pos) => (&content[..pos], content[pos..].trim()),
        None => (content, ""),
    };

    let name = name.trim().to_ascii_lowercase();
    if name.is_empty() {
        return None;
    }

    Some((
        Tag {
            name,
            attrs: attrs.to_string(),
            is_close,
            self_closing,
        },
        end,
    ))
}

/// Extract href value from tag attribute string.
fn extract_href(attrs: &str) -> Option<String> {
    let idx = attrs.find("href=")?;
    let rest = &attrs[idx + 5..];
    let bytes = rest.as_bytes();
    if bytes.is_empty() {
        return None;
    }

    if bytes[0] == b'"' || bytes[0] == b'\'' {
        let quote = bytes[0];
        let end = bytes[1..].iter().position(|&b| b == quote)? + 1;
        Some(rest[1..end].to_string())
    } else {
        // Unquoted: read until whitespace or end
        let end = rest
            .find(|c: char| c.is_whitespace() || c == '>')
            .unwrap_or(rest.len());
        Some(rest[..end].to_string())
    }
}

/// Decode an HTML entity starting at `&` (position `start`).
/// Returns `(decoded_char, bytes_consumed)`.
fn decode_entity(bytes: &[u8], start: usize) -> Option<(char, usize)> {
    if start >= bytes.len() || bytes[start] != b'&' {
        return None;
    }

    let rest = &bytes[start..];
    let semi = rest.iter().position(|&b| b == b';')?;
    if semi > 10 {
        // Entity too long, not valid
        return None;
    }

    let entity = std::str::from_utf8(&rest[..semi + 1]).ok()?;
    let consumed = semi + 1;

    match entity {
        "&amp;" => Some(('&', consumed)),
        "&lt;" => Some(('<', consumed)),
        "&gt;" => Some(('>', consumed)),
        "&quot;" => Some(('"', consumed)),
        "&#39;" => Some(('\'', consumed)),
        "&apos;" => Some(('\'', consumed)),
        "&nbsp;" => Some(('\u{00A0}', consumed)),
        _ => {
            // Try numeric: &#NNN; or &#xHHH;
            if entity.starts_with("&#x") || entity.starts_with("&#X") {
                let hex = &entity[3..entity.len() - 1];
                let code = u32::from_str_radix(hex, 16).ok()?;
                char::from_u32(code).map(|c| (c, consumed))
            } else if entity.starts_with("&#") {
                let num = &entity[2..entity.len() - 1];
                let code: u32 = num.parse().ok()?;
                char::from_u32(code).map(|c| (c, consumed))
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::style::Color;

    fn base() -> Style {
        Style::default().fg(Color::White)
    }

    /// Helper: flatten all spans in all lines into a single string.
    fn text(lines: &[Line<'_>]) -> String {
        lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Helper: get the style of the first non-indent span containing `needle`.
    fn style_of(lines: &[Line<'_>], needle: &str) -> Option<Style> {
        for line in lines {
            for span in &line.spans {
                if span.content.contains(needle) {
                    return Some(span.style);
                }
            }
        }
        None
    }

    #[test]
    fn plain_text_no_tags() {
        let lines = html_to_lines("hello world", base(), 0);
        assert_eq!(lines.len(), 1);
        assert_eq!(text(&lines), "hello world");
    }

    #[test]
    fn empty_input() {
        let lines = html_to_lines("", base(), 0);
        assert!(lines.is_empty());
    }

    #[test]
    fn bold_format() {
        let lines = html_to_lines("a <b>bold</b> z", base(), 0);
        let s = style_of(&lines, "bold").unwrap();
        assert!(s.add_modifier.contains(Modifier::BOLD));
        assert_eq!(text(&lines), "a bold z");
    }

    #[test]
    fn italic_format() {
        let lines = html_to_lines("a <i>italic</i> z", base(), 0);
        let s = style_of(&lines, "italic").unwrap();
        assert!(s.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn strikethrough_format() {
        let lines = html_to_lines("a <del>deleted</del> z", base(), 0);
        let s = style_of(&lines, "deleted").unwrap();
        assert!(s.add_modifier.contains(Modifier::CROSSED_OUT));
    }

    #[test]
    fn strong_and_em_aliases() {
        let lines = html_to_lines("<strong>b</strong> <em>i</em> <s>s</s>", base(), 0);
        assert!(
            style_of(&lines, "b")
                .unwrap()
                .add_modifier
                .contains(Modifier::BOLD)
        );
        assert!(
            style_of(&lines, "i")
                .unwrap()
                .add_modifier
                .contains(Modifier::ITALIC)
        );
        assert!(
            style_of(&lines, "s")
                .unwrap()
                .add_modifier
                .contains(Modifier::CROSSED_OUT)
        );
    }

    #[test]
    fn nested_bold_italic() {
        let lines = html_to_lines("<b><i>both</i></b>", base(), 0);
        let s = style_of(&lines, "both").unwrap();
        assert!(s.add_modifier.contains(Modifier::BOLD));
        assert!(s.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn inline_code_style() {
        let lines = html_to_lines("a <code>fn main</code> z", base(), 0);
        let s = style_of(&lines, "fn main").unwrap();
        assert_eq!(s.fg, Some(theme::CODE_INLINE_FG));
        assert_eq!(s.bg, Some(theme::CODE_INLINE_BG));
    }

    #[test]
    fn pre_preserves_newlines() {
        let lines = html_to_lines("<pre><code>line1\nline2\nline3</code></pre>", base(), 0);
        assert!(lines.len() >= 3, "expected >=3 lines, got {}", lines.len());
        let t = text(&lines);
        assert!(t.contains("line1"));
        assert!(t.contains("line2"));
        assert!(t.contains("line3"));
    }

    #[test]
    fn pre_code_uses_block_style() {
        let lines = html_to_lines("<pre><code>code</code></pre>", base(), 0);
        let s = style_of(&lines, "code").unwrap();
        // Should use code_block_style, not code_inline_style
        assert_eq!(s.bg, Some(theme::CODE_BLOCK_BG));
        assert_ne!(s.bg, Some(theme::CODE_INLINE_BG));
    }

    #[test]
    fn link_with_different_url() {
        let lines = html_to_lines(r#"<a href="https://example.com">click</a>"#, base(), 0);
        let t = text(&lines);
        assert!(t.contains("click"));
        assert!(t.contains("(https://example.com)"));
    }

    #[test]
    fn link_with_same_url() {
        let lines = html_to_lines(
            r#"<a href="https://example.com">https://example.com</a>"#,
            base(),
            0,
        );
        let t = text(&lines);
        // Should NOT duplicate the URL
        assert_eq!(t.matches("https://example.com").count(), 1);
    }

    #[test]
    fn br_creates_new_line() {
        let lines = html_to_lines("hello<br>world", base(), 0);
        assert_eq!(lines.len(), 2);
        let t = text(&lines);
        assert!(t.contains("hello"));
        assert!(t.contains("world"));
    }

    #[test]
    fn p_creates_new_line() {
        let lines = html_to_lines("<p>para1</p><p>para2</p>", base(), 0);
        assert!(lines.len() >= 2);
        let t = text(&lines);
        assert!(t.contains("para1"));
        assert!(t.contains("para2"));
    }

    #[test]
    fn blockquote_has_prefix() {
        let lines = html_to_lines("<blockquote>quoted text</blockquote>", base(), 0);
        let t = text(&lines);
        assert!(t.contains("│ "));
        assert!(t.contains("quoted text"));
    }

    #[test]
    fn unordered_list_bullets() {
        let lines = html_to_lines("<ul><li>a</li><li>b</li></ul>", base(), 0);
        let t = text(&lines);
        assert!(t.contains("• a"));
        assert!(t.contains("• b"));
    }

    #[test]
    fn ordered_list_numbers() {
        let lines = html_to_lines("<ol><li>first</li><li>second</li></ol>", base(), 0);
        let t = text(&lines);
        assert!(t.contains("1. first"));
        assert!(t.contains("2. second"));
    }

    #[test]
    fn h1_bold_uppercase() {
        let lines = html_to_lines("<h1>title</h1>", base(), 0);
        let t = text(&lines);
        assert!(t.contains("TITLE"));
    }

    #[test]
    fn h2_bold_only() {
        let lines = html_to_lines("<h2>subtitle</h2>", base(), 0);
        let t = text(&lines);
        assert!(t.contains("subtitle"));
        // Should NOT be uppercase
        assert!(!t.contains("SUBTITLE"));
        let s = style_of(&lines, "subtitle").unwrap();
        assert!(s.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn mx_reply_content_skipped() {
        let lines = html_to_lines(
            "<mx-reply><blockquote>reply</blockquote></mx-reply>actual message",
            base(),
            0,
        );
        let t = text(&lines);
        assert!(!t.contains("reply"));
        assert!(t.contains("actual message"));
    }

    #[test]
    fn html_entities_decoded() {
        let lines = html_to_lines("&amp; &lt; &gt; &quot; &#39;", base(), 0);
        let t = text(&lines);
        assert!(t.contains("& < > \" '"));
    }

    #[test]
    fn continuation_line_indentation() {
        let lines = html_to_lines("first<br>second<br>third", base(), 4);
        assert_eq!(lines.len(), 3);
        // First line: no indent
        let first_text: String = lines[0].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(!first_text.starts_with("    "));
        // Second line: indented
        let second_text: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(second_text.starts_with("    "));
    }

    #[test]
    fn unclosed_tag_no_panic() {
        let lines = html_to_lines("<b>bold text without close", base(), 0);
        let t = text(&lines);
        assert!(t.contains("bold text without close"));
    }

    #[test]
    fn extra_close_tag_ignored() {
        let lines = html_to_lines("text</b></i></unknown>more", base(), 0);
        let t = text(&lines);
        assert!(t.contains("text"));
        assert!(t.contains("more"));
    }

    #[test]
    fn lone_less_than_handled() {
        let lines = html_to_lines("a < b and c > d", base(), 0);
        // The '<' should be preserved somehow (might grab some text as a tag attempt)
        assert!(!lines.is_empty());
    }

    #[test]
    fn whitespace_collapsed_outside_pre() {
        let lines = html_to_lines("hello   world\n\nnewline", base(), 0);
        let t = text(&lines);
        assert!(t.contains("hello world"));
    }

    #[test]
    fn nested_blockquote() {
        let lines = html_to_lines(
            "<blockquote><blockquote>deep</blockquote></blockquote>",
            base(),
            0,
        );
        // Should have double blockquote prefix
        let deep_line = lines
            .iter()
            .find(|l| l.spans.iter().any(|s| s.content.contains("deep")));
        assert!(deep_line.is_some());
        // Count "│ " prefixes
        let prefix_count = deep_line
            .unwrap()
            .spans
            .iter()
            .filter(|s| s.content.as_ref() == "│ ")
            .count();
        assert_eq!(prefix_count, 2);
    }

    #[test]
    fn link_style_applied() {
        let lines = html_to_lines(r#"<a href="https://example.com">link</a>"#, base(), 0);
        let s = style_of(&lines, "link").unwrap();
        assert_eq!(s.fg, Some(theme::LINK_FG));
        assert!(s.add_modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn blockquote_style_applied() {
        let lines = html_to_lines("<blockquote>quoted</blockquote>", base(), 0);
        let s = style_of(&lines, "quoted").unwrap();
        assert_eq!(s.fg, Some(theme::BLOCKQUOTE_FG));
    }

    #[test]
    fn base_style_propagated() {
        let custom = Style::default().fg(Color::Green);
        let lines = html_to_lines("hello", custom, 0);
        let s = style_of(&lines, "hello").unwrap();
        assert_eq!(s.fg, Some(Color::Green));
    }

    #[test]
    fn style_reverts_after_close_tag() {
        let lines = html_to_lines("<b>bold</b> normal", base(), 0);
        let bold_style = style_of(&lines, "bold").unwrap();
        assert!(bold_style.add_modifier.contains(Modifier::BOLD));
        let normal_style = style_of(&lines, " normal").unwrap();
        assert!(!normal_style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn numeric_decimal_entity() {
        // &#60; = '<', &#62; = '>'
        let lines = html_to_lines("&#60;div&#62;", base(), 0);
        let t = text(&lines);
        assert!(t.contains("<div>"));
    }

    #[test]
    fn numeric_hex_entity() {
        // &#x3C; = '<', &#x3E; = '>'
        let lines = html_to_lines("&#x3C;span&#x3E;", base(), 0);
        let t = text(&lines);
        assert!(t.contains("<span>"));
    }

    #[test]
    fn unknown_entity_passthrough() {
        let lines = html_to_lines("&unknownentity; text", base(), 0);
        let t = text(&lines);
        // decode_entity returns None, so '&' is literal, rest is text
        assert!(t.contains('&'));
        assert!(t.contains("text"));
        assert!(!lines.is_empty());
    }

    #[test]
    fn self_closing_br() {
        let lines = html_to_lines("hello<br/>world", base(), 0);
        assert_eq!(lines.len(), 2);
        let t = text(&lines);
        assert!(t.contains("hello"));
        assert!(t.contains("world"));
    }

    #[test]
    fn case_insensitive_tags() {
        let lines = html_to_lines("<B>bold</B> <STRONG>strong</STRONG>", base(), 0);
        assert!(
            style_of(&lines, "bold")
                .unwrap()
                .add_modifier
                .contains(Modifier::BOLD)
        );
        assert!(
            style_of(&lines, "strong")
                .unwrap()
                .add_modifier
                .contains(Modifier::BOLD)
        );
    }

    #[test]
    fn unicode_multibyte_text() {
        let lines = html_to_lines("héllo 🌍 wörld", base(), 0);
        let t = text(&lines);
        assert!(t.contains("héllo"));
        assert!(t.contains("🌍"));
        assert!(t.contains("wörld"));
    }

    #[test]
    fn pre_preserves_spaces_and_tabs() {
        let lines = html_to_lines("<pre>  a\tb</pre>", base(), 0);
        let t = text(&lines);
        assert!(t.contains("  a\tb"));
    }

    #[test]
    fn empty_tags_no_artifacts() {
        let lines = html_to_lines("before<b></b>after", base(), 0);
        let t = text(&lines);
        assert_eq!(t, "beforeafter");
    }

    #[test]
    fn nested_lists() {
        let lines = html_to_lines(
            "<ul><li>outer<ol><li>inner1</li><li>inner2</li></ol></li></ul>",
            base(),
            0,
        );
        let t = text(&lines);
        assert!(t.contains("• outer"));
        assert!(t.contains("1. inner1"));
        assert!(t.contains("2. inner2"));
    }

    #[test]
    fn link_no_href() {
        let lines = html_to_lines("<a>text</a>", base(), 0);
        let t = text(&lines);
        assert!(t.contains("text"));
        // Should not have empty parens
        assert!(!t.contains("()"));
    }

    #[test]
    fn realistic_matrix_html() {
        let html = concat!(
            "<mx-reply><blockquote>quoted reply</blockquote></mx-reply>",
            "<p>Hello <b>world</b>, check <code>foo()</code> and ",
            r#"<a href="https://example.com">this link</a>.</p>"#,
            "<p>Second paragraph with &amp; entity.</p>"
        );
        let lines = html_to_lines(html, base(), 4);
        let t = text(&lines);
        // mx-reply skipped
        assert!(!t.contains("quoted reply"));
        // Content present
        assert!(t.contains("Hello"));
        assert!(t.contains("world"));
        assert!(t.contains("foo()"));
        assert!(t.contains("this link"));
        assert!(t.contains("(https://example.com)"));
        assert!(t.contains("& entity"));
        // Bold applied
        assert!(
            style_of(&lines, "world")
                .unwrap()
                .add_modifier
                .contains(Modifier::BOLD)
        );
        // Inline code style applied
        assert_eq!(
            style_of(&lines, "foo()").unwrap().fg,
            Some(theme::CODE_INLINE_FG)
        );
    }

    #[test]
    fn tag_with_extra_attributes() {
        let lines = html_to_lines(r#"<p class="foo" id="bar">text</p>"#, base(), 0);
        let t = text(&lines);
        assert!(t.contains("text"));
    }
}
