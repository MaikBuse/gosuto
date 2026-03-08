#[derive(Debug, Clone)]
pub enum AuthState {
    LoggedOut,
    LoggingIn,
    AutoLoggingIn,
    Registering,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logged_out_is_not_logged_in() {
        assert!(!AuthState::LoggedOut.is_logged_in());
    }

    #[test]
    fn logging_in_is_not_logged_in() {
        assert!(!AuthState::LoggingIn.is_logged_in());
    }

    #[test]
    fn auto_logging_in_is_not_logged_in() {
        assert!(!AuthState::AutoLoggingIn.is_logged_in());
    }

    #[test]
    fn registering_is_not_logged_in() {
        assert!(!AuthState::Registering.is_logged_in());
    }

    #[test]
    fn logged_in_is_logged_in() {
        let state = AuthState::LoggedIn {
            user_id: "@user:example.com".to_string(),
            device_id: "DEVICE".to_string(),
            homeserver: "https://example.com".to_string(),
        };
        assert!(state.is_logged_in());
    }

    #[test]
    fn error_is_not_logged_in() {
        assert!(!AuthState::Error("something failed".to_string()).is_logged_in());
    }
}
