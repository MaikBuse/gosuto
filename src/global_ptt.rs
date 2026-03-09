use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::Mutex;

use tracing::{info, warn};

use crate::event::{AppEvent, EventSender};

#[cfg(target_os = "linux")]
pub fn check_linux_prerequisites() -> Option<String> {
    let has_input_access = std::fs::read_dir("/dev/input")
        .ok()
        .and_then(|entries| {
            entries
                .filter_map(Result::ok)
                .find(|e| e.file_name().to_string_lossy().starts_with("event"))
                .map(|e| std::fs::File::open(e.path()).is_ok())
        })
        .unwrap_or(false);

    if !has_input_access {
        return Some(
            "PTT requires /dev/input access. Add your user to the `input` group: sudo usermod -aG input $USER (then re-login)".to_string()
        );
    }

    None
}

#[cfg(not(target_os = "linux"))]
pub fn check_linux_prerequisites() -> Option<String> {
    None
}

pub struct GlobalPttHandle {
    pub active: Arc<AtomicBool>,
    pub capturing: Arc<AtomicBool>,
    pub ptt_key: Arc<Mutex<String>>,
    pub alive: Arc<AtomicBool>,
}

pub fn spawn_listener(
    ptt_transmitting: Arc<AtomicBool>,
    ptt_key: String,
    event_tx: EventSender,
) -> GlobalPttHandle {
    let active = Arc::new(AtomicBool::new(false));
    let capturing = Arc::new(AtomicBool::new(false));
    let ptt_key_shared = Arc::new(Mutex::new(ptt_key));
    let alive = Arc::new(AtomicBool::new(true));

    let handle = GlobalPttHandle {
        active: active.clone(),
        capturing: capturing.clone(),
        ptt_key: ptt_key_shared.clone(),
        alive: alive.clone(),
    };

    std::thread::Builder::new()
        .name("ptt-listener".into())
        .spawn(move || {
            info!("PTT listener thread started");
            let error_tx = event_tx.clone();

            let failed = spawn_listener_inner(
                &active,
                &capturing,
                &ptt_key_shared,
                &ptt_transmitting,
                &event_tx,
            );

            info!("PTT listener loop exited");
            alive.store(false, Ordering::Relaxed);
            if let Some(err) = failed {
                let message = if cfg!(target_os = "linux") {
                    format!("PTT failed: {err}. Add your user to the `input` group.")
                } else if cfg!(target_os = "macos") {
                    format!(
                        "PTT failed: {err}. Grant Accessibility permission in System Settings.",
                    )
                } else {
                    format!("PTT failed: {err}")
                };
                warn!("{}", message);
                let _ = error_tx.send(AppEvent::PttListenerFailed(message));
            }
        })
        .expect("failed to spawn PTT listener thread");

    handle
}

/// Callback logic shared between Linux (evdev) and non-Linux (rdev) paths.
fn handle_ptt_event(
    key_name: &str,
    is_press: bool,
    active: &AtomicBool,
    capturing: &AtomicBool,
    ptt_key_shared: &Mutex<String>,
    ptt_transmitting: &AtomicBool,
    event_tx: &EventSender,
) {
    if is_press {
        if capturing.load(Ordering::Relaxed) {
            let _ = event_tx.send(AppEvent::PttKeyCaptured(key_name.to_string()));
            capturing.store(false, Ordering::Relaxed);
        } else if active.load(Ordering::Relaxed) {
            let current_key = ptt_key_shared.lock().clone();
            if !current_key.is_empty() && key_name == current_key {
                ptt_transmitting.store(true, Ordering::Relaxed);
            }
        }
    } else if active.load(Ordering::Relaxed) {
        let current_key = ptt_key_shared.lock().clone();
        if !current_key.is_empty() && key_name == current_key {
            ptt_transmitting.store(false, Ordering::Relaxed);
        }
    }
}

// ── Linux: evdev-based passive listener ──────────────────────────────────────

#[cfg(target_os = "linux")]
fn spawn_listener_inner(
    active: &AtomicBool,
    capturing: &AtomicBool,
    ptt_key_shared: &Mutex<String>,
    ptt_transmitting: &AtomicBool,
    event_tx: &EventSender,
) -> Option<String> {
    info!("Using evdev (passive) backend for PTT");
    evdev_listen(|key_name, is_press| {
        handle_ptt_event(
            key_name,
            is_press,
            active,
            capturing,
            ptt_key_shared,
            ptt_transmitting,
            event_tx,
        );
    })
    .err()
}

#[cfg(target_os = "linux")]
fn evdev_listen(mut callback: impl FnMut(&str, bool)) -> Result<(), String> {
    use evdev::{Device, EventSummary, KeyCode};
    use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
    use std::os::fd::AsFd;

    let mut devices: Vec<Device> = evdev::enumerate()
        .map(|(_path, dev)| dev)
        .filter(|dev| {
            let keys = dev.supported_keys();
            keys.is_some_and(|k| k.contains(KeyCode::KEY_A))
        })
        .collect();

    if devices.is_empty() {
        return Err("No keyboard devices found in /dev/input".to_string());
    }

    info!("Monitoring {} keyboard device(s) via evdev", devices.len());

    // Set all devices to non-blocking
    for dev in &mut devices {
        dev.set_nonblocking(true)
            .map_err(|e| format!("Failed to set non-blocking: {e}"))?;
    }

    loop {
        // Build poll fds fresh each iteration (devices may be removed)
        let mut poll_fds: Vec<PollFd<'_>> = devices
            .iter()
            .map(|d| PollFd::new(d.as_fd(), PollFlags::POLLIN))
            .collect();

        match poll(&mut poll_fds, PollTimeout::NONE) {
            Ok(0) => continue,
            Err(nix::errno::Errno::EINTR) => continue,
            Err(e) => return Err(format!("poll() failed: {e}")),
            Ok(_) => {}
        }

        // Collect ready/error indices, then drop poll_fds to release the borrow
        let mut ready_indices = Vec::new();
        let mut to_remove = Vec::new();

        for (i, pfd) in poll_fds.iter().enumerate() {
            let revents = pfd.revents().unwrap_or(PollFlags::empty());

            if revents.intersects(PollFlags::POLLHUP | PollFlags::POLLERR) {
                warn!("Input device {} disconnected", i);
                to_remove.push(i);
            } else if revents.contains(PollFlags::POLLIN) {
                ready_indices.push(i);
            }
        }
        drop(poll_fds);

        for i in ready_indices {
            match devices[i].fetch_events() {
                Ok(events) => {
                    for ev in events {
                        // value: 0 = release, 1 = press, 2 = repeat
                        if let EventSummary::Key(_, key, value) = ev.destructure()
                            && (value == 0 || value == 1)
                            && let Some(name) = evdev_key_to_name(key)
                        {
                            callback(name, value == 1);
                        }
                    }
                }
                Err(e) if e.raw_os_error() == Some(19) => {
                    // ENODEV — device removed
                    warn!("Input device {} removed (ENODEV)", i);
                    to_remove.push(i);
                }
                Err(_) => {
                    // Transient read error, skip
                }
            }
        }

        // Remove disconnected devices in reverse order to preserve indices
        to_remove.dedup();
        for idx in to_remove.into_iter().rev() {
            devices.remove(idx);
        }

        if devices.is_empty() {
            return Err("All keyboard devices disconnected".to_string());
        }
    }
}

#[cfg(target_os = "linux")]
fn evdev_key_to_name(key: evdev::KeyCode) -> Option<&'static str> {
    use evdev::KeyCode;

    let name = match key {
        KeyCode::KEY_LEFTALT | KeyCode::KEY_RIGHTALT => "Alt",
        KeyCode::KEY_LEFTCTRL | KeyCode::KEY_RIGHTCTRL => "Ctrl",
        KeyCode::KEY_LEFTSHIFT | KeyCode::KEY_RIGHTSHIFT => "Shift",
        KeyCode::KEY_LEFTMETA | KeyCode::KEY_RIGHTMETA => "Super",
        KeyCode::KEY_SPACE => "Space",
        KeyCode::KEY_ENTER => "Enter",
        KeyCode::KEY_TAB => "Tab",
        KeyCode::KEY_BACKSPACE => "Backspace",
        KeyCode::KEY_ESC => "Esc",
        KeyCode::KEY_LEFT => "Left",
        KeyCode::KEY_RIGHT => "Right",
        KeyCode::KEY_UP => "Up",
        KeyCode::KEY_DOWN => "Down",
        KeyCode::KEY_HOME => "Home",
        KeyCode::KEY_END => "End",
        KeyCode::KEY_PAGEUP => "PageUp",
        KeyCode::KEY_PAGEDOWN => "PageDown",
        KeyCode::KEY_INSERT => "Insert",
        KeyCode::KEY_DELETE => "Delete",
        KeyCode::KEY_F1 => "F1",
        KeyCode::KEY_F2 => "F2",
        KeyCode::KEY_F3 => "F3",
        KeyCode::KEY_F4 => "F4",
        KeyCode::KEY_F5 => "F5",
        KeyCode::KEY_F6 => "F6",
        KeyCode::KEY_F7 => "F7",
        KeyCode::KEY_F8 => "F8",
        KeyCode::KEY_F9 => "F9",
        KeyCode::KEY_F10 => "F10",
        KeyCode::KEY_F11 => "F11",
        KeyCode::KEY_F12 => "F12",
        KeyCode::KEY_A => "A",
        KeyCode::KEY_B => "B",
        KeyCode::KEY_C => "C",
        KeyCode::KEY_D => "D",
        KeyCode::KEY_E => "E",
        KeyCode::KEY_F => "F",
        KeyCode::KEY_G => "G",
        KeyCode::KEY_H => "H",
        KeyCode::KEY_I => "I",
        KeyCode::KEY_J => "J",
        KeyCode::KEY_K => "K",
        KeyCode::KEY_L => "L",
        KeyCode::KEY_M => "M",
        KeyCode::KEY_N => "N",
        KeyCode::KEY_O => "O",
        KeyCode::KEY_P => "P",
        KeyCode::KEY_Q => "Q",
        KeyCode::KEY_R => "R",
        KeyCode::KEY_S => "S",
        KeyCode::KEY_T => "T",
        KeyCode::KEY_U => "U",
        KeyCode::KEY_V => "V",
        KeyCode::KEY_W => "W",
        KeyCode::KEY_X => "X",
        KeyCode::KEY_Y => "Y",
        KeyCode::KEY_Z => "Z",
        KeyCode::KEY_0 => "0",
        KeyCode::KEY_1 => "1",
        KeyCode::KEY_2 => "2",
        KeyCode::KEY_3 => "3",
        KeyCode::KEY_4 => "4",
        KeyCode::KEY_5 => "5",
        KeyCode::KEY_6 => "6",
        KeyCode::KEY_7 => "7",
        KeyCode::KEY_8 => "8",
        KeyCode::KEY_9 => "9",
        _ => return None,
    };
    Some(name)
}

// ── Non-Linux: rdev-based listener ───────────────────────────────────────────

#[cfg(not(target_os = "linux"))]
fn spawn_listener_inner(
    active: &AtomicBool,
    capturing: &AtomicBool,
    ptt_key_shared: &Mutex<String>,
    ptt_transmitting: &AtomicBool,
    event_tx: &EventSender,
) -> Option<String> {
    use rdev::EventType;

    info!("Using rdev (listen) backend for PTT");
    rdev::listen(move |event: rdev::Event| match event.event_type {
        EventType::KeyPress(key) => {
            if let Some(name) = rdev_key_to_name(key) {
                handle_ptt_event(
                    &name,
                    true,
                    active,
                    capturing,
                    ptt_key_shared,
                    ptt_transmitting,
                    event_tx,
                );
            }
        }
        EventType::KeyRelease(key) => {
            if let Some(name) = rdev_key_to_name(key) {
                handle_ptt_event(
                    &name,
                    false,
                    active,
                    capturing,
                    ptt_key_shared,
                    ptt_transmitting,
                    event_tx,
                );
            }
        }
        _ => {}
    })
    .err()
    .map(|e| format!("{e:?}"))
}

#[cfg(not(target_os = "linux"))]
fn rdev_key_to_name(key: rdev::Key) -> Option<String> {
    use rdev::Key;

    let name = match key {
        Key::Alt | Key::AltGr => "Alt",
        Key::ControlLeft | Key::ControlRight => "Ctrl",
        Key::ShiftLeft | Key::ShiftRight => "Shift",
        Key::MetaLeft | Key::MetaRight => "Super",
        Key::Space => "Space",
        Key::Return => "Enter",
        Key::Tab => "Tab",
        Key::Backspace => "Backspace",
        Key::Escape => "Esc",
        Key::LeftArrow => "Left",
        Key::RightArrow => "Right",
        Key::UpArrow => "Up",
        Key::DownArrow => "Down",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",
        Key::Insert => "Insert",
        Key::Delete => "Delete",
        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",
        Key::KeyA => "A",
        Key::KeyB => "B",
        Key::KeyC => "C",
        Key::KeyD => "D",
        Key::KeyE => "E",
        Key::KeyF => "F",
        Key::KeyG => "G",
        Key::KeyH => "H",
        Key::KeyI => "I",
        Key::KeyJ => "J",
        Key::KeyK => "K",
        Key::KeyL => "L",
        Key::KeyM => "M",
        Key::KeyN => "N",
        Key::KeyO => "O",
        Key::KeyP => "P",
        Key::KeyQ => "Q",
        Key::KeyR => "R",
        Key::KeyS => "S",
        Key::KeyT => "T",
        Key::KeyU => "U",
        Key::KeyV => "V",
        Key::KeyW => "W",
        Key::KeyX => "X",
        Key::KeyY => "Y",
        Key::KeyZ => "Z",
        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        _ => return None,
    };
    Some(name.to_string())
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
    mod linux_tests {
        use super::*;

        #[test]
        fn evdev_modifier_keys_mapped_correctly() {
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_LEFTCTRL).unwrap(),
                "Ctrl"
            );
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_RIGHTCTRL).unwrap(),
                "Ctrl"
            );
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_LEFTSHIFT).unwrap(),
                "Shift"
            );
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_RIGHTSHIFT).unwrap(),
                "Shift"
            );
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_LEFTALT).unwrap(),
                "Alt"
            );
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_RIGHTALT).unwrap(),
                "Alt"
            );
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_LEFTMETA).unwrap(),
                "Super"
            );
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_RIGHTMETA).unwrap(),
                "Super"
            );
        }

        #[test]
        fn evdev_letter_keys_uppercase() {
            assert_eq!(evdev_key_to_name(evdev::KeyCode::KEY_A).unwrap(), "A");
            assert_eq!(evdev_key_to_name(evdev::KeyCode::KEY_Z).unwrap(), "Z");
        }

        #[test]
        fn evdev_special_keys_mapped() {
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_SPACE).unwrap(),
                "Space"
            );
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_ENTER).unwrap(),
                "Enter"
            );
            assert_eq!(evdev_key_to_name(evdev::KeyCode::KEY_TAB).unwrap(), "Tab");
            assert_eq!(evdev_key_to_name(evdev::KeyCode::KEY_ESC).unwrap(), "Esc");
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_BACKSPACE).unwrap(),
                "Backspace"
            );
        }

        #[test]
        fn evdev_function_keys_mapped() {
            assert_eq!(evdev_key_to_name(evdev::KeyCode::KEY_F1).unwrap(), "F1");
            assert_eq!(evdev_key_to_name(evdev::KeyCode::KEY_F12).unwrap(), "F12");
        }

        #[test]
        fn evdev_number_keys_mapped() {
            assert_eq!(evdev_key_to_name(evdev::KeyCode::KEY_0).unwrap(), "0");
            assert_eq!(evdev_key_to_name(evdev::KeyCode::KEY_9).unwrap(), "9");
        }

        #[test]
        fn evdev_navigation_keys_mapped() {
            assert_eq!(evdev_key_to_name(evdev::KeyCode::KEY_LEFT).unwrap(), "Left");
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_RIGHT).unwrap(),
                "Right"
            );
            assert_eq!(evdev_key_to_name(evdev::KeyCode::KEY_UP).unwrap(), "Up");
            assert_eq!(evdev_key_to_name(evdev::KeyCode::KEY_DOWN).unwrap(), "Down");
            assert_eq!(evdev_key_to_name(evdev::KeyCode::KEY_HOME).unwrap(), "Home");
            assert_eq!(evdev_key_to_name(evdev::KeyCode::KEY_END).unwrap(), "End");
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_PAGEUP).unwrap(),
                "PageUp"
            );
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_PAGEDOWN).unwrap(),
                "PageDown"
            );
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_INSERT).unwrap(),
                "Insert"
            );
            assert_eq!(
                evdev_key_to_name(evdev::KeyCode::KEY_DELETE).unwrap(),
                "Delete"
            );
        }

        #[test]
        fn evdev_unknown_key_returns_none() {
            assert!(evdev_key_to_name(evdev::KeyCode::KEY_SYSRQ).is_none());
        }
    }

    #[cfg(not(target_os = "linux"))]
    mod rdev_tests {
        use super::*;

        #[test]
        fn modifier_keys_mapped_correctly() {
            assert_eq!(rdev_key_to_name(rdev::Key::ControlLeft).unwrap(), "Ctrl");
            assert_eq!(rdev_key_to_name(rdev::Key::ControlRight).unwrap(), "Ctrl");
            assert_eq!(rdev_key_to_name(rdev::Key::ShiftLeft).unwrap(), "Shift");
            assert_eq!(rdev_key_to_name(rdev::Key::ShiftRight).unwrap(), "Shift");
            assert_eq!(rdev_key_to_name(rdev::Key::Alt).unwrap(), "Alt");
            assert_eq!(rdev_key_to_name(rdev::Key::AltGr).unwrap(), "Alt");
            assert_eq!(rdev_key_to_name(rdev::Key::MetaLeft).unwrap(), "Super");
            assert_eq!(rdev_key_to_name(rdev::Key::MetaRight).unwrap(), "Super");
        }

        #[test]
        fn letter_keys_uppercase() {
            assert_eq!(rdev_key_to_name(rdev::Key::KeyA).unwrap(), "A");
            assert_eq!(rdev_key_to_name(rdev::Key::KeyZ).unwrap(), "Z");
        }

        #[test]
        fn special_keys_mapped() {
            assert_eq!(rdev_key_to_name(rdev::Key::Space).unwrap(), "Space");
            assert_eq!(rdev_key_to_name(rdev::Key::Return).unwrap(), "Enter");
            assert_eq!(rdev_key_to_name(rdev::Key::Tab).unwrap(), "Tab");
            assert_eq!(rdev_key_to_name(rdev::Key::Escape).unwrap(), "Esc");
            assert_eq!(rdev_key_to_name(rdev::Key::Backspace).unwrap(), "Backspace");
        }

        #[test]
        fn function_keys_mapped() {
            assert_eq!(rdev_key_to_name(rdev::Key::F1).unwrap(), "F1");
            assert_eq!(rdev_key_to_name(rdev::Key::F12).unwrap(), "F12");
        }

        #[test]
        fn number_keys_mapped() {
            assert_eq!(rdev_key_to_name(rdev::Key::Num0).unwrap(), "0");
            assert_eq!(rdev_key_to_name(rdev::Key::Num9).unwrap(), "9");
        }

        #[test]
        fn navigation_keys_mapped() {
            assert_eq!(rdev_key_to_name(rdev::Key::LeftArrow).unwrap(), "Left");
            assert_eq!(rdev_key_to_name(rdev::Key::RightArrow).unwrap(), "Right");
            assert_eq!(rdev_key_to_name(rdev::Key::UpArrow).unwrap(), "Up");
            assert_eq!(rdev_key_to_name(rdev::Key::DownArrow).unwrap(), "Down");
            assert_eq!(rdev_key_to_name(rdev::Key::Home).unwrap(), "Home");
            assert_eq!(rdev_key_to_name(rdev::Key::End).unwrap(), "End");
            assert_eq!(rdev_key_to_name(rdev::Key::PageUp).unwrap(), "PageUp");
            assert_eq!(rdev_key_to_name(rdev::Key::PageDown).unwrap(), "PageDown");
            assert_eq!(rdev_key_to_name(rdev::Key::Insert).unwrap(), "Insert");
            assert_eq!(rdev_key_to_name(rdev::Key::Delete).unwrap(), "Delete");
        }

        #[test]
        fn unknown_key_returns_none() {
            assert!(rdev_key_to_name(rdev::Key::Unknown(0xFFFF)).is_none());
        }
    }
}
