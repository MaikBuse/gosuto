use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
pub enum MessageContent {
    Text(String),
    Image {
        body: String,
        width: Option<u32>,
        height: Option<u32>,
    },
}

#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub event_id: String,
    pub sender: String,
    pub content: MessageContent,
    pub timestamp: DateTime<Local>,
    pub is_emote: bool,
    pub is_notice: bool,
    pub pending: bool,
    pub verified: Option<bool>,
}

impl DisplayMessage {
    pub fn body_text(&self) -> &str {
        match &self.content {
            MessageContent::Text(s) => s,
            MessageContent::Image { body, .. } => body,
        }
    }
}

#[derive(Debug)]
pub struct MessageState {
    pub messages: Vec<DisplayMessage>,
    pub scroll_offset: usize,
    pub has_more: bool,
    pub loading: bool,
    pub fetch_error: Option<String>,
    pub current_room_id: Option<String>,
    pub selected_index: Option<usize>,
}

impl MessageState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            has_more: true,
            loading: false,
            fetch_error: None,
            current_room_id: None,
            selected_index: None,
        }
    }

    pub fn set_room(&mut self, room_id: Option<String>) {
        if self.current_room_id != room_id {
            self.current_room_id = room_id;
            self.messages.clear();
            self.scroll_offset = 0;
            self.has_more = true;
            self.loading = true;
            self.fetch_error = None;
            self.selected_index = None;
        }
    }

    pub fn add_message(&mut self, msg: DisplayMessage) {
        // Check if this message already exists (by event_id)
        if !msg.event_id.is_empty() && self.messages.iter().any(|m| m.event_id == msg.event_id) {
            return;
        }
        self.messages.push(msg);
    }

    pub fn prepend_messages(&mut self, msgs: Vec<DisplayMessage>, has_more: bool) {
        // Filter out messages that already exist (by event_id) to avoid duplicates
        // when sync events overlap with fetched messages
        let mut new_msgs: Vec<DisplayMessage> = msgs
            .into_iter()
            .filter(|m| {
                m.event_id.is_empty()
                    || !self
                        .messages
                        .iter()
                        .any(|existing| existing.event_id == m.event_id)
            })
            .collect();
        let prepended_count = new_msgs.len();
        new_msgs.append(&mut self.messages);
        self.messages = new_msgs;
        if let Some(idx) = self.selected_index {
            self.selected_index = Some(idx + prepended_count);
        }
        self.has_more = has_more;
        self.loading = false;
        self.fetch_error = None;
    }

    pub fn set_fetch_error(&mut self, error: String) {
        self.fetch_error = Some(error);
        self.loading = false;
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn is_selecting(&self) -> bool {
        self.selected_index.is_some()
    }

    pub fn select_newest(&mut self) {
        if !self.messages.is_empty() {
            self.selected_index = Some(self.messages.len() - 1);
        }
    }

    pub fn deselect(&mut self) {
        self.selected_index = None;
    }

    pub fn select_up(&mut self) {
        if let Some(idx) = self.selected_index {
            self.selected_index = Some(idx.saturating_sub(1));
        }
    }

    pub fn select_down(&mut self) {
        if let Some(idx) = self.selected_index {
            let max = self.messages.len().saturating_sub(1);
            self.selected_index = Some((idx + 1).min(max));
        }
    }

    pub fn select_top(&mut self) {
        if self.selected_index.is_some() {
            self.selected_index = Some(0);
        }
    }

    pub fn select_bottom(&mut self) {
        if self.selected_index.is_some() && !self.messages.is_empty() {
            self.selected_index = Some(self.messages.len() - 1);
        }
    }

    pub fn confirm_sent(&mut self, pending_body: &str, event_id: &str) {
        if let Some(msg) = self
            .messages
            .iter_mut()
            .rev()
            .find(|m| m.pending && m.body_text() == pending_body)
        {
            msg.pending = false;
            msg.event_id = event_id.to_string();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(event_id: &str, body: &str, pending: bool) -> DisplayMessage {
        DisplayMessage {
            event_id: event_id.to_string(),
            sender: "@user:example.com".to_string(),
            content: MessageContent::Text(body.to_string()),
            timestamp: Local::now(),
            is_emote: false,
            is_notice: false,
            pending,
            verified: None,
        }
    }

    #[test]
    fn add_message_with_unique_event_id() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "hello", false));
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].body_text(), "hello");
    }

    #[test]
    fn add_message_dedup_same_event_id() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "hello", false));
        state.add_message(make_msg("$1", "hello again", false));
        assert_eq!(state.messages.len(), 1);
    }

    #[test]
    fn add_message_empty_event_id_always_added() {
        let mut state = MessageState::new();
        state.add_message(make_msg("", "first", false));
        state.add_message(make_msg("", "second", false));
        assert_eq!(state.messages.len(), 2);
    }

    #[test]
    fn prepend_messages_filters_duplicates() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "existing", false));
        state.prepend_messages(
            vec![make_msg("$1", "dup", false), make_msg("$2", "new", false)],
            true,
        );
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.messages[0].body_text(), "new");
        assert_eq!(state.messages[1].body_text(), "existing");
    }

    #[test]
    fn prepend_messages_ordering() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$3", "C", false));
        state.prepend_messages(
            vec![make_msg("$1", "A", false), make_msg("$2", "B", false)],
            false,
        );
        assert_eq!(state.messages[0].body_text(), "A");
        assert_eq!(state.messages[1].body_text(), "B");
        assert_eq!(state.messages[2].body_text(), "C");
    }

    #[test]
    fn prepend_messages_sets_has_more() {
        let mut state = MessageState::new();
        state.prepend_messages(vec![make_msg("$1", "msg", false)], false);
        assert!(!state.has_more);
        state.prepend_messages(vec![make_msg("$2", "msg2", false)], true);
        assert!(state.has_more);
    }

    #[test]
    fn prepend_messages_clears_loading() {
        let mut state = MessageState::new();
        state.loading = true;
        state.prepend_messages(vec![], false);
        assert!(!state.loading);
    }

    #[test]
    fn confirm_sent_matches_pending_by_body() {
        let mut state = MessageState::new();
        state.add_message(make_msg("", "hello world", true));
        state.confirm_sent("hello world", "$evt1");
        assert!(!state.messages[0].pending);
        assert_eq!(state.messages[0].event_id, "$evt1");
    }

    #[test]
    fn confirm_sent_no_match_leaves_unchanged() {
        let mut state = MessageState::new();
        state.add_message(make_msg("", "hello", true));
        state.confirm_sent("no match", "$evt1");
        assert!(state.messages[0].pending);
        assert_eq!(state.messages[0].event_id, "");
    }

    #[test]
    fn set_room_clears_state() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "msg", false));
        state.scroll_offset = 5;
        state.has_more = false;
        state.set_room(Some("!room1:example.com".to_string()));
        assert!(state.messages.is_empty());
        assert_eq!(state.scroll_offset, 0);
        assert!(state.has_more);
        assert!(state.loading);
    }

    #[test]
    fn set_room_noop_on_same_room() {
        let mut state = MessageState::new();
        state.set_room(Some("!room1:example.com".to_string()));
        state.add_message(make_msg("$1", "msg", false));
        state.loading = false;
        state.set_room(Some("!room1:example.com".to_string()));
        assert_eq!(state.messages.len(), 1);
        assert!(!state.loading);
    }

    #[test]
    fn scroll_to_bottom_resets() {
        let mut state = MessageState::new();
        state.scroll_offset = 10;
        state.scroll_to_bottom();
        assert_eq!(state.scroll_offset, 0);
    }

    #[test]
    fn set_fetch_error_stores_error_and_clears_loading() {
        let mut state = MessageState::new();
        state.loading = true;
        state.set_fetch_error("network error".to_string());
        assert_eq!(state.fetch_error.as_deref(), Some("network error"));
        assert!(!state.loading);
    }

    // --- Selection mode ---

    #[test]
    fn select_newest_selects_last_message() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "A", false));
        state.add_message(make_msg("$2", "B", false));
        state.select_newest();
        assert_eq!(state.selected_index, Some(1));
    }

    #[test]
    fn select_newest_noop_when_empty() {
        let mut state = MessageState::new();
        state.select_newest();
        assert_eq!(state.selected_index, None);
    }

    #[test]
    fn deselect_clears_selection() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "A", false));
        state.select_newest();
        assert!(state.is_selecting());
        state.deselect();
        assert!(!state.is_selecting());
        assert_eq!(state.selected_index, None);
    }

    #[test]
    fn select_up_decrements() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "A", false));
        state.add_message(make_msg("$2", "B", false));
        state.selected_index = Some(1);
        state.select_up();
        assert_eq!(state.selected_index, Some(0));
    }

    #[test]
    fn select_up_saturates_at_zero() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "A", false));
        state.selected_index = Some(0);
        state.select_up();
        assert_eq!(state.selected_index, Some(0));
    }

    #[test]
    fn select_down_increments() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "A", false));
        state.add_message(make_msg("$2", "B", false));
        state.selected_index = Some(0);
        state.select_down();
        assert_eq!(state.selected_index, Some(1));
    }

    #[test]
    fn select_down_clamps_at_last() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "A", false));
        state.selected_index = Some(0);
        state.select_down();
        assert_eq!(state.selected_index, Some(0));
    }

    #[test]
    fn select_top_jumps_to_first() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "A", false));
        state.add_message(make_msg("$2", "B", false));
        state.selected_index = Some(1);
        state.select_top();
        assert_eq!(state.selected_index, Some(0));
    }

    #[test]
    fn select_top_noop_when_not_selecting() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "A", false));
        state.select_top();
        assert_eq!(state.selected_index, None);
    }

    #[test]
    fn select_bottom_jumps_to_last() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "A", false));
        state.add_message(make_msg("$2", "B", false));
        state.selected_index = Some(0);
        state.select_bottom();
        assert_eq!(state.selected_index, Some(1));
    }

    #[test]
    fn select_bottom_noop_when_not_selecting() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "A", false));
        state.select_bottom();
        assert_eq!(state.selected_index, None);
    }

    #[test]
    fn prepend_messages_adjusts_selected_index() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$3", "C", false));
        state.selected_index = Some(0);
        state.prepend_messages(
            vec![make_msg("$1", "A", false), make_msg("$2", "B", false)],
            true,
        );
        // Original index 0 should shift by 2
        assert_eq!(state.selected_index, Some(2));
    }

    #[test]
    fn prepend_messages_preserves_none_selection() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$2", "B", false));
        state.prepend_messages(vec![make_msg("$1", "A", false)], true);
        assert_eq!(state.selected_index, None);
    }

    #[test]
    fn set_room_clears_selection() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$1", "msg", false));
        state.select_newest();
        assert!(state.is_selecting());
        state.set_room(Some("!room2:example.com".to_string()));
        assert_eq!(state.selected_index, None);
    }
}
