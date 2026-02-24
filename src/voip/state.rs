use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallState {
    /// We are sending an invite, waiting for answer
    Inviting,
    /// Incoming call ringing, waiting for user to accept/reject
    Ringing,
    /// SDP/ICE exchange in progress
    Connecting,
    /// Call is active with audio flowing
    Active,
}

#[derive(Debug, Clone)]
pub struct CallInfo {
    pub call_id: String,
    pub room_id: String,
    pub remote_user: String,
    pub state: CallState,
    pub started_at: Option<Instant>,
}

impl CallInfo {
    pub fn new_outgoing(call_id: String, room_id: String, remote_user: String) -> Self {
        Self {
            call_id,
            room_id,
            remote_user,
            state: CallState::Inviting,
            started_at: None,
        }
    }

    pub fn new_incoming(call_id: String, room_id: String, remote_user: String) -> Self {
        Self {
            call_id,
            room_id,
            remote_user,
            state: CallState::Ringing,
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
