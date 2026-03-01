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
