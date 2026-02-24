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
        if !msg.event_id.is_empty()
            && self.messages.iter().any(|m| m.event_id == msg.event_id)
        {
            return;
        }
        self.messages.push(msg);
    }

    pub fn prepend_messages(&mut self, msgs: Vec<DisplayMessage>, has_more: bool) {
        let mut new_msgs = msgs;
        new_msgs.extend(self.messages.drain(..));
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
