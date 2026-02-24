#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AuthState {
    LoggedOut,
    LoggingIn,
    LoggedIn {
        user_id: String,
        device_id: String,
        homeserver: String,
    },
    Error(String),
}

impl AuthState {
    pub fn is_logged_in(&self) -> bool {
        matches!(self, AuthState::LoggedIn { .. })
    }
}
