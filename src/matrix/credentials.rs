use tracing::warn;

fn service_name() -> String {
    match crate::config::active_profile() {
        Some(name) => format!("gosuto-{name}"),
        None => "gosuto".to_owned(),
    }
}

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
        if let Err(e) =
            keyring::Entry::new(&service_name(), key).and_then(|entry| entry.set_password(value))
        {
            warn!("Failed to save {key} to keyring: {e}");
            return;
        }
    }
}

pub fn load_credentials() -> Option<SavedCredentials> {
    let get = |key: &str| -> Option<String> {
        keyring::Entry::new(&service_name(), key)
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
        if let Ok(entry) = keyring::Entry::new(&service_name(), key)
            && let Err(e) = entry.delete_credential()
        {
            tracing::warn!("Failed to delete keyring credential '{key}': {e}");
        }
    }
}
