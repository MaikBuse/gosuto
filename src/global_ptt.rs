use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rdev::{EventType, Key};
use tracing::warn;

use crate::event::{AppEvent, EventSender};

pub struct GlobalPttHandle {
    pub active: Arc<AtomicBool>,
    pub capturing: Arc<AtomicBool>,
    pub ptt_key: Arc<Mutex<String>>,
}

pub fn spawn_listener(
    ptt_transmitting: Arc<AtomicBool>,
    ptt_key: String,
    event_tx: EventSender,
) -> GlobalPttHandle {
    let active = Arc::new(AtomicBool::new(false));
    let capturing = Arc::new(AtomicBool::new(false));
    let ptt_key_shared = Arc::new(Mutex::new(ptt_key));

    let handle = GlobalPttHandle {
        active: active.clone(),
        capturing: capturing.clone(),
        ptt_key: ptt_key_shared.clone(),
    };

    tokio::task::spawn_blocking(move || {
        let result = rdev::listen(move |event| match event.event_type {
            EventType::KeyPress(key) => {
                let Some(name) = rdev_key_to_name(key) else {
                    return;
                };
                if capturing.load(Ordering::Relaxed) {
                    let _ = event_tx.send(AppEvent::PttKeyCaptured(name));
                    capturing.store(false, Ordering::Relaxed);
                } else if active.load(Ordering::Relaxed) {
                    let current_key = ptt_key_shared.lock().unwrap().clone();
                    if !current_key.is_empty() && name == current_key {
                        ptt_transmitting.store(true, Ordering::Relaxed);
                    }
                }
            }
            EventType::KeyRelease(key) => {
                if let Some(name) = rdev_key_to_name(key) {
                    let current_key = ptt_key_shared.lock().unwrap().clone();
                    if !current_key.is_empty() && name == current_key {
                        ptt_transmitting.store(false, Ordering::Relaxed);
                    }
                }
            }
            _ => {}
        });

        if let Err(e) = result {
            if cfg!(target_os = "linux") {
                warn!(
                    "Global PTT listener failed: {:?}. Try adding your user to the `input` group.",
                    e
                );
            } else if cfg!(target_os = "macos") {
                warn!(
                    "Global PTT listener failed: {:?}. Grant Accessibility permission in System Settings.",
                    e
                );
            } else {
                warn!("Global PTT listener failed: {:?}", e);
            }
        }
    });

    handle
}

fn rdev_key_to_name(key: Key) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifier_keys_mapped_correctly() {
        assert_eq!(rdev_key_to_name(Key::ControlLeft).unwrap(), "Ctrl");
        assert_eq!(rdev_key_to_name(Key::ControlRight).unwrap(), "Ctrl");
        assert_eq!(rdev_key_to_name(Key::ShiftLeft).unwrap(), "Shift");
        assert_eq!(rdev_key_to_name(Key::ShiftRight).unwrap(), "Shift");
        assert_eq!(rdev_key_to_name(Key::Alt).unwrap(), "Alt");
        assert_eq!(rdev_key_to_name(Key::AltGr).unwrap(), "Alt");
        assert_eq!(rdev_key_to_name(Key::MetaLeft).unwrap(), "Super");
        assert_eq!(rdev_key_to_name(Key::MetaRight).unwrap(), "Super");
    }

    #[test]
    fn letter_keys_uppercase() {
        assert_eq!(rdev_key_to_name(Key::KeyA).unwrap(), "A");
        assert_eq!(rdev_key_to_name(Key::KeyZ).unwrap(), "Z");
    }

    #[test]
    fn special_keys_mapped() {
        assert_eq!(rdev_key_to_name(Key::Space).unwrap(), "Space");
        assert_eq!(rdev_key_to_name(Key::Return).unwrap(), "Enter");
        assert_eq!(rdev_key_to_name(Key::Tab).unwrap(), "Tab");
        assert_eq!(rdev_key_to_name(Key::Escape).unwrap(), "Esc");
        assert_eq!(rdev_key_to_name(Key::Backspace).unwrap(), "Backspace");
    }

    #[test]
    fn function_keys_mapped() {
        assert_eq!(rdev_key_to_name(Key::F1).unwrap(), "F1");
        assert_eq!(rdev_key_to_name(Key::F12).unwrap(), "F12");
    }

    #[test]
    fn number_keys_mapped() {
        assert_eq!(rdev_key_to_name(Key::Num0).unwrap(), "0");
        assert_eq!(rdev_key_to_name(Key::Num9).unwrap(), "9");
    }

    #[test]
    fn navigation_keys_mapped() {
        assert_eq!(rdev_key_to_name(Key::LeftArrow).unwrap(), "Left");
        assert_eq!(rdev_key_to_name(Key::RightArrow).unwrap(), "Right");
        assert_eq!(rdev_key_to_name(Key::UpArrow).unwrap(), "Up");
        assert_eq!(rdev_key_to_name(Key::DownArrow).unwrap(), "Down");
        assert_eq!(rdev_key_to_name(Key::Home).unwrap(), "Home");
        assert_eq!(rdev_key_to_name(Key::End).unwrap(), "End");
        assert_eq!(rdev_key_to_name(Key::PageUp).unwrap(), "PageUp");
        assert_eq!(rdev_key_to_name(Key::PageDown).unwrap(), "PageDown");
        assert_eq!(rdev_key_to_name(Key::Insert).unwrap(), "Insert");
        assert_eq!(rdev_key_to_name(Key::Delete).unwrap(), "Delete");
    }

    #[test]
    fn unknown_key_returns_none() {
        assert!(rdev_key_to_name(Key::Unknown(0xFFFF)).is_none());
    }
}
