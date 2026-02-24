use tracing::warn;

const SERVICE: &str = "walrust";

pub struct SavedCredentials {
    pub homeserver: String,
    pub username: String,
    pub password: String,
}

pub fn save_credentials(homeserver: &str, username: &str, password: &str) {
    for (key, value) in [
        ("homeserver", homeserver),
        ("username", username),
        ("password", password),
    ] {
        if let Err(e) = keyring::Entry::new(SERVICE, key)
            .and_then(|entry| entry.set_password(value))
        {
            warn!("Failed to save {key} to keyring: {e}");
            return;
        }
    }
}

pub fn load_credentials() -> Option<SavedCredentials> {
    let get = |key: &str| -> Option<String> {
        keyring::Entry::new(SERVICE, key)
            .and_then(|entry| entry.get_password())
            .ok()
    };

    Some(SavedCredentials {
        homeserver: get("homeserver")?,
        username: get("username")?,
        password: get("password")?,
    })
}

pub fn delete_credentials() {
    for key in ["homeserver", "username", "password"] {
        if let Ok(entry) = keyring::Entry::new(SERVICE, key) {
            let _ = entry.delete_credential();
        }
    }
}
