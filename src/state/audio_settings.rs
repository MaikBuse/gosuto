use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crossterm::event::{KeyCode, KeyEvent};

pub struct AudioSettingsState {
    pub open: bool,
    pub selected_field: usize,
    pub input_devices: Vec<String>,
    pub output_devices: Vec<String>,
    pub input_device_idx: usize,
    pub output_device_idx: usize,
    pub input_volume: f32,
    pub output_volume: f32,
    pub voice_activity: bool,
    pub sensitivity: f32,
    pub push_to_talk: bool,
    pub push_to_talk_key: Option<String>,
    pub capturing_ptt_key: bool,
    pub ptt_error: Option<String>,
    pub vad_hold_ms: u64,
    pub mic_level: f32,
    pub mic_test_running: Arc<AtomicBool>,
}

impl AudioSettingsState {
    pub fn new() -> Self {
        Self {
            open: false,
            selected_field: 0,
            input_devices: vec!["Default".to_string()],
            output_devices: vec!["Default".to_string()],
            input_device_idx: 0,
            output_device_idx: 0,
            input_volume: 1.0,
            output_volume: 1.0,
            voice_activity: false,
            sensitivity: 0.15,
            push_to_talk: false,
            push_to_talk_key: None,
            capturing_ptt_key: false,
            ptt_error: None,
            vad_hold_ms: 300,
            mic_level: 0.0,
            mic_test_running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn visible_fields(&self) -> Vec<usize> {
        let mut fields = vec![0, 1, 2, 3, 4];
        if self.voice_activity {
            fields.push(5); // sensitivity
            fields.push(8); // vad hold
        }
        fields.push(6);
        if self.push_to_talk {
            fields.push(7);
        }
        fields
    }

    pub fn current_field(&self) -> usize {
        let visible = self.visible_fields();
        visible.get(self.selected_field).copied().unwrap_or(0)
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AudioSettingsAction {
        // Swallow terminal keys while rdev captures the PTT key
        if self.capturing_ptt_key {
            return AudioSettingsAction::None;
        }

        let visible = self.visible_fields();
        let max_sel = visible.len().saturating_sub(1);

        match key.code {
            KeyCode::Esc => AudioSettingsAction::Close,
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected_field = (self.selected_field + 1).min(max_sel);
                AudioSettingsAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_field = self.selected_field.saturating_sub(1);
                AudioSettingsAction::None
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.adjust_field(-1);
                if self.current_field() == 0 {
                    AudioSettingsAction::StartMicTest
                } else {
                    AudioSettingsAction::None
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.adjust_field(1);
                if self.current_field() == 0 {
                    AudioSettingsAction::StartMicTest
                } else {
                    AudioSettingsAction::None
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let field = self.current_field();
                match field {
                    4 => {
                        self.voice_activity = !self.voice_activity;
                        if self.voice_activity {
                            self.push_to_talk = false;
                        }
                        AudioSettingsAction::ToggleVad
                    }
                    6 => {
                        self.push_to_talk = !self.push_to_talk;
                        if self.push_to_talk {
                            self.voice_activity = false;
                        }
                        AudioSettingsAction::TogglePtt
                    }
                    7 => {
                        self.ptt_error = None;
                        AudioSettingsAction::CapturePttKey
                    }
                    _ => AudioSettingsAction::None,
                }
            }
            _ => AudioSettingsAction::None,
        }
    }

    fn adjust_field(&mut self, dir: i32) {
        let field = self.current_field();
        match field {
            0 => {
                // Input device
                let len = self.input_devices.len();
                if dir > 0 {
                    self.input_device_idx = (self.input_device_idx + 1) % len;
                } else {
                    self.input_device_idx = (self.input_device_idx + len - 1) % len;
                }
            }
            1 => {
                // Output device
                let len = self.output_devices.len();
                if dir > 0 {
                    self.output_device_idx = (self.output_device_idx + 1) % len;
                } else {
                    self.output_device_idx = (self.output_device_idx + len - 1) % len;
                }
            }
            2 => {
                // Input volume
                let step = 0.05;
                self.input_volume = (self.input_volume + dir as f32 * step).clamp(0.0, 1.0);
            }
            3 => {
                // Output volume
                let step = 0.05;
                self.output_volume = (self.output_volume + dir as f32 * step).clamp(0.0, 1.0);
            }
            4 => {
                // Voice activity toggle
                self.voice_activity = dir > 0;
                if self.voice_activity {
                    self.push_to_talk = false;
                }
            }
            5 => {
                // Sensitivity
                let step = 0.05;
                self.sensitivity = (self.sensitivity + dir as f32 * step).clamp(0.0, 1.0);
            }
            6 => {
                // Push to talk toggle
                self.push_to_talk = dir > 0;
                if self.push_to_talk {
                    self.voice_activity = false;
                }
            }
            8 => {
                // VAD hold ms
                let step: i64 = 50;
                let new_val = (self.vad_hold_ms as i64 + i64::from(dir) * step).clamp(0, 1000);
                self.vad_hold_ms = new_val as u64;
            }
            _ => {}
        }
    }
}

pub enum AudioSettingsAction {
    None,
    Close,
    StartMicTest,
    CapturePttKey,
    ToggleVad,
    TogglePtt,
}
