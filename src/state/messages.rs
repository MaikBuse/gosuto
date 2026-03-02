use chrono::{DateTime, Local};

#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub event_id: String,
    pub sender: String,
    pub body: String,
    pub timestamp: DateTime<Local>,
    pub is_emote: bool,
    pub is_notice: bool,
    pub pending: bool,
    pub verified: Option<bool>,
}

#[derive(Debug)]
pub struct MessageState {
    pub messages: Vec<DisplayMessage>,
    pub scroll_offset: usize,
    pub has_more: bool,
    pub loading: bool,
    pub fetch_error: Option<String>,
    pub current_room_id: Option<String>,
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
        new_msgs.append(&mut self.messages);
        self.messages = new_msgs;
        self.has_more = has_more;
        self.loading = false;
        self.fetch_error = None;
    }

    pub fn set_fetch_error(&mut self, error: String) {
        self.fetch_error = Some(error);
        self.loading = false;
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_add(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn confirm_sent(&mut self, pending_body: &str, event_id: &str) {
        if let Some(msg) = self
            .messages
            .iter_mut()
            .rev()
            .find(|m| m.pending && m.body == pending_body)
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
            body: body.to_string(),
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
        assert_eq!(state.messages[0].body, "hello");
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
        assert_eq!(state.messages[0].body, "new");
        assert_eq!(state.messages[1].body, "existing");
    }

    #[test]
    fn prepend_messages_ordering() {
        let mut state = MessageState::new();
        state.add_message(make_msg("$3", "C", false));
        state.prepend_messages(
            vec![make_msg("$1", "A", false), make_msg("$2", "B", false)],
            false,
        );
        assert_eq!(state.messages[0].body, "A");
        assert_eq!(state.messages[1].body, "B");
        assert_eq!(state.messages[2].body, "C");
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
    fn scroll_up_increments() {
        let mut state = MessageState::new();
        assert_eq!(state.scroll_offset, 0);
        state.scroll_up();
        assert_eq!(state.scroll_offset, 1);
        state.scroll_up();
        assert_eq!(state.scroll_offset, 2);
    }

    #[test]
    fn scroll_down_decrements() {
        let mut state = MessageState::new();
        state.scroll_offset = 3;
        state.scroll_down();
        assert_eq!(state.scroll_offset, 2);
    }

    #[test]
    fn scroll_down_saturates_at_zero() {
        let mut state = MessageState::new();
        state.scroll_down();
        assert_eq!(state.scroll_offset, 0);
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
}
