use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallState {
    /// Acquiring JWT + connecting to LiveKit
    Connecting,
    /// Connected, audio flowing
    Active,
}

#[derive(Debug, Clone)]
pub struct CallInfo {
    pub room_id: String,
    pub room_name: Option<String>,
    pub state: CallState,
    pub is_incoming: bool,
    pub participants: Vec<String>,
    pub started_at: Option<Instant>,
}

impl CallInfo {
    pub fn new_outgoing(room_id: String, room_name: Option<String>) -> Self {
        Self {
            room_id,
            room_name,
            state: CallState::Connecting,
            is_incoming: false,
            participants: Vec::new(),
            started_at: None,
        }
    }

    pub fn new_incoming(room_id: String, caller: String, room_name: Option<String>) -> Self {
        Self {
            room_id,
            room_name,
            state: CallState::Connecting,
            is_incoming: true,
            participants: vec![caller],
            started_at: None,
        }
    }

    /// Returns elapsed seconds since call became active
    pub fn elapsed_secs(&self) -> Option<u64> {
        self.started_at.map(|t| t.elapsed().as_secs())
    }

    /// Format elapsed time as MM:SS
    pub fn elapsed_display(&self) -> String {
        match self.elapsed_secs() {
            Some(secs) => format!("{:02}:{:02}", secs / 60, secs % 60),
            None => "--:--".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_outgoing_fields() {
        let call = CallInfo::new_outgoing("!room:x".to_string(), Some("Room".to_string()));
        assert_eq!(call.room_id, "!room:x");
        assert_eq!(call.room_name.as_deref(), Some("Room"));
        assert_eq!(call.state, CallState::Connecting);
        assert!(!call.is_incoming);
        assert!(call.participants.is_empty());
        assert!(call.started_at.is_none());
    }

    #[test]
    fn new_outgoing_no_room_name() {
        let call = CallInfo::new_outgoing("!room:x".to_string(), None);
        assert!(call.room_name.is_none());
    }

    #[test]
    fn new_incoming_fields() {
        let call = CallInfo::new_incoming(
            "!room:x".to_string(),
            "@caller:x".to_string(),
            Some("Room".to_string()),
        );
        assert_eq!(call.room_id, "!room:x");
        assert!(call.is_incoming);
        assert_eq!(call.state, CallState::Connecting);
        assert_eq!(call.participants, vec!["@caller:x"]);
    }

    #[test]
    fn elapsed_display_no_start() {
        let call = CallInfo::new_outgoing("!room:x".to_string(), None);
        assert_eq!(call.elapsed_display(), "--:--");
    }

    #[test]
    fn elapsed_display_with_start() {
        let mut call = CallInfo::new_outgoing("!room:x".to_string(), None);
        call.started_at = Some(Instant::now());
        // Should show 00:00 or 00:01 depending on timing
        let display = call.elapsed_display();
        assert!(display.contains(':'));
        assert_eq!(display.len(), 5); // "MM:SS" format
    }

    #[test]
    fn elapsed_secs_none_when_not_started() {
        let call = CallInfo::new_outgoing("!room:x".to_string(), None);
        assert!(call.elapsed_secs().is_none());
    }
}
