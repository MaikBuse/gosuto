use super::*;

impl App {
    pub(crate) fn open_audio_settings(&mut self) {
        // Enumerate devices
        let mut input_devices = vec!["Default".to_string()];
        input_devices.extend(AudioPipeline::enumerate_input_devices());
        let mut output_devices = vec!["Default".to_string()];
        output_devices.extend(AudioPipeline::enumerate_output_devices());

        // Find current device indices
        let input_idx = self
            .config
            .audio
            .input_device
            .as_ref()
            .and_then(|name| input_devices.iter().position(|d| d == name))
            .unwrap_or(0);
        let output_idx = self
            .config
            .audio
            .output_device
            .as_ref()
            .and_then(|name| output_devices.iter().position(|d| d == name))
            .unwrap_or(0);

        self.audio_settings = AudioSettingsState {
            open: true,
            selected_field: 0,
            input_devices,
            output_devices,
            input_device_idx: input_idx,
            output_device_idx: output_idx,
            input_volume: self.config.audio.input_volume,
            output_volume: self.config.audio.output_volume,
            voice_activity: self.config.audio.voice_activity,
            sensitivity: self.config.audio.sensitivity,
            push_to_talk: self.config.audio.push_to_talk,
            push_to_talk_key: self.config.audio.push_to_talk_key.clone(),
            capturing_ptt_key: false,
            ptt_error: None,
            vad_hold_ms: self.config.audio.vad_hold_ms,
            mic_level: 0.0,
            mic_test_running: Arc::new(AtomicBool::new(false)),
        };

        // Start mic test
        self.start_mic_test();
    }

    pub(crate) fn close_audio_settings(&mut self) {
        // Stop mic test
        self.audio_settings
            .mic_test_running
            .store(false, Ordering::Relaxed);

        // Sync state back to config
        let s = &self.audio_settings;
        self.config.audio.input_device = if s.input_device_idx == 0 {
            None
        } else {
            s.input_devices.get(s.input_device_idx).cloned()
        };
        self.config.audio.output_device = if s.output_device_idx == 0 {
            None
        } else {
            s.output_devices.get(s.output_device_idx).cloned()
        };
        self.config.audio.input_volume = s.input_volume;
        self.config.audio.output_volume = s.output_volume;
        self.config.audio.voice_activity = s.voice_activity;
        self.config.audio.sensitivity = s.sensitivity;
        self.config.audio.push_to_talk = s.push_to_talk;
        self.config.audio.push_to_talk_key = s.push_to_talk_key.clone();
        self.config.audio.vad_hold_ms = s.vad_hold_ms;

        // Update PTT transmitting default
        if !self.config.audio.push_to_talk {
            self.ptt_transmitting.store(true, Ordering::Relaxed);
        } else {
            self.ptt_transmitting.store(false, Ordering::Relaxed);
        }

        // Spawn global PTT listener on demand, or sync key if already running
        if self.config.audio.push_to_talk {
            self.ensure_global_ptt_listener();
            if let Some(ref handle) = self.global_ptt {
                *handle.ptt_key.lock() = self
                    .config
                    .audio
                    .push_to_talk_key
                    .clone()
                    .unwrap_or_default();
            }
        }

        crate::config::save_config(&self.config);
        self.audio_settings.open = false;
    }

    pub fn start_mic_test(&mut self) {
        // Stop any existing mic test (old Arc stays false, old thread exits)
        self.audio_settings
            .mic_test_running
            .store(false, Ordering::Relaxed);

        // Create a fresh running flag for the new test
        let running = Arc::new(AtomicBool::new(true));
        self.audio_settings.mic_test_running = running.clone();

        let device_name = if self.audio_settings.input_device_idx == 0 {
            None
        } else {
            self.audio_settings
                .input_devices
                .get(self.audio_settings.input_device_idx)
                .cloned()
        };
        let volume = self.audio_settings.input_volume;
        let tx = self.event_tx.clone();

        std::thread::spawn(move || {
            if let Err(e) =
                AudioPipeline::start_mic_test(device_name.as_deref(), volume, tx, running)
            {
                error!("Mic test error: {}", e);
            }
        });
    }

    pub(crate) fn handle_audio_settings_key(&mut self, key: KeyEvent) {
        // Ctrl+C always quits
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.close_audio_settings();
            self.running = false;
            return;
        }

        match self.audio_settings.handle_key(key) {
            AudioSettingsAction::None => {}
            AudioSettingsAction::Close => {
                self.close_audio_settings();
            }
            AudioSettingsAction::StartMicTest => {
                self.start_mic_test();
            }
            AudioSettingsAction::CapturePttKey => {
                self.ensure_global_ptt_listener();
                if let Some(ref handle) = self.global_ptt {
                    self.audio_settings.capturing_ptt_key = true;
                    handle.capturing.store(true, Ordering::Relaxed);
                }
            }
            AudioSettingsAction::ToggleVad | AudioSettingsAction::TogglePtt => {}
        }
    }

    pub(crate) fn ensure_global_ptt_listener(&mut self) {
        if let Some(ref handle) = self.global_ptt
            && !handle.alive.load(Ordering::Relaxed)
        {
            self.global_ptt = None;
        }
        if self.global_ptt.is_none() {
            if let Some(error) = crate::global_ptt::check_linux_prerequisites() {
                self.audio_settings.ptt_error = Some(error);
                return;
            }
            let ptt_key = self
                .config
                .audio
                .push_to_talk_key
                .clone()
                .unwrap_or_default();
            let handle = crate::global_ptt::spawn_listener(
                self.ptt_transmitting.clone(),
                ptt_key,
                self.event_tx.clone(),
            );
            self.global_ptt = Some(handle);
        }
    }

    pub(crate) fn set_global_ptt_active(&self, active: bool) {
        if let Some(ref handle) = self.global_ptt {
            handle.active.store(active, Ordering::Relaxed);
        }
    }
}
