use super::*;
use crate::config::GosutoConfig;
use crate::event::AppEvent;
use crate::state::{HealingStep, RecoveryStage};

fn test_app() -> App {
    let (event_tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let config = GosutoConfig::default();
    let picker = ratatui_image::picker::Picker::halfblocks();
    let (image_decode_tx, _image_decode_rx) = std::sync::mpsc::channel();
    App::new(event_tx, config, picker, image_decode_tx)
}

#[test]
fn user_config_loaded_sets_display_name_when_open() {
    let mut app = test_app();
    app.user_config.open = true;
    app.user_config.loading = true;

    app.handle_event(AppEvent::UserConfigLoaded {
        display_name: Some("Alice".to_string()),
        verified: true,
        recovery_status: crate::event::RecoveryStatus::Disabled,
    });

    assert!(!app.user_config.loading);
    assert_eq!(app.user_config.display_name, Some("Alice".to_string()));
    assert!(app.self_verified);
    assert!(app.user_config.verified);
}

#[test]
fn user_config_loaded_sets_self_verified_when_modal_closed() {
    let mut app = test_app();
    assert!(!app.self_verified);
    assert!(!app.user_config.open);

    app.handle_event(AppEvent::UserConfigLoaded {
        display_name: None,
        verified: true,
        recovery_status: crate::event::RecoveryStatus::Disabled,
    });

    assert!(app.self_verified);
    assert!(!app.user_config.loading);
}

#[test]
fn full_restart_flow_sets_verified_before_modal_open() {
    let mut app = test_app();
    assert!(!app.self_verified);
    assert!(app.sync_token.is_none());

    // First sync triggers pending fetch
    app.handle_event(AppEvent::SyncTokenUpdated("tok1".to_string()));
    assert!(app.pending_user_config);

    // Main loop consumes the flag
    app.pending_user_config = false;

    // SDK responds with verified=true
    app.handle_event(AppEvent::UserConfigLoaded {
        display_name: Some("Bob".to_string()),
        verified: true,
        recovery_status: crate::event::RecoveryStatus::Disabled,
    });
    assert!(app.self_verified);

    // User opens :configure — verified should propagate from self_verified
    app.user_config = UserConfigState {
        open: true,
        verified: app.self_verified,
        loading: true,
        ..UserConfigState::new()
    };
    assert!(app.user_config.verified);
}

#[test]
fn logout_resets_self_verified() {
    let mut app = test_app();
    app.self_verified = true;
    app.auto_login_attempted = true; // prevent auto-login side effects

    app.handle_event(AppEvent::LoggedOut);

    assert!(!app.self_verified);
}

#[test]
fn first_sync_token_triggers_user_config_fetch() {
    let mut app = test_app();
    assert!(app.sync_token.is_none());

    app.handle_event(AppEvent::SyncTokenUpdated("tok1".to_string()));

    assert!(app.pending_user_config);
    assert_eq!(app.sync_token, Some("tok1".to_string()));
}

#[test]
fn subsequent_sync_token_does_not_trigger_fetch() {
    let mut app = test_app();
    app.sync_token = Some("tok1".to_string());
    app.pending_user_config = false;

    app.handle_event(AppEvent::SyncTokenUpdated("tok2".to_string()));

    assert!(!app.pending_user_config);
    assert_eq!(app.sync_token, Some("tok2".to_string()));
}

#[test]
fn recovery_command_opens_modal() {
    let mut app = test_app();
    app.auth = crate::state::AuthState::LoggedIn {
        user_id: "@test:example.com".to_string(),
        device_id: "DEV".to_string(),
        homeserver: "https://example.com".to_string(),
    };
    app.handle_command(CommandAction::Recovery);
    assert!(app.recovery.is_some());
    assert_eq!(app.pending_recovery, Some(RecoveryAction::Check));
}

#[test]
fn recovery_event_updates_stage() {
    let mut app = test_app();
    app.recovery = Some(RecoveryModalState::new());

    app.handle_event(AppEvent::RecoveryStateChecked(RecoveryStage::Enabled));
    assert_eq!(app.recovery.as_ref().unwrap().stage, RecoveryStage::Enabled);

    app.handle_event(AppEvent::RecoveryKeyReady("key123".to_string()));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::ShowKey("key123".to_string())
    );

    app.recovery = Some(RecoveryModalState::new());
    app.handle_event(AppEvent::RecoveryRecovered);
    assert_eq!(app.recovery.as_ref().unwrap().stage, RecoveryStage::Enabled);

    app.recovery = Some(RecoveryModalState::new());
    app.handle_event(AppEvent::RecoveryError("bad".to_string()));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::Error("bad".to_string())
    );
}

#[test]
fn healing_progress_updates_stage() {
    let mut app = test_app();
    app.recovery = Some(RecoveryModalState::new());

    app.handle_event(AppEvent::RecoveryHealingProgress(HealingStep::CrossSigning));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::Healing(HealingStep::CrossSigning)
    );

    app.handle_event(AppEvent::RecoveryHealingProgress(HealingStep::Backup));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::Healing(HealingStep::Backup)
    );

    app.handle_event(AppEvent::RecoveryHealingProgress(
        HealingStep::ExportSecrets,
    ));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::Healing(HealingStep::ExportSecrets)
    );
}

#[test]
fn need_password_event_sets_stage() {
    use crate::event::PasswordSender;

    let mut app = test_app();
    app.recovery = Some(RecoveryModalState::new());

    let (tx, _rx) = tokio::sync::oneshot::channel();
    app.handle_event(AppEvent::RecoveryNeedPassword(PasswordSender::new(tx)));

    let modal = app.recovery.as_ref().unwrap();
    assert_eq!(modal.stage, RecoveryStage::NeedPassword);
    assert!(modal.password_tx.is_some());
    assert!(modal.password_buffer.is_empty());
}

#[test]
fn healing_skips_cross_signing_starts_at_backup() {
    let mut app = test_app();
    app.recovery = Some(RecoveryModalState::new());
    app.recovery.as_mut().unwrap().stage = RecoveryStage::Recovering;

    app.handle_event(AppEvent::RecoveryHealingProgress(HealingStep::Backup));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::Healing(HealingStep::Backup)
    );
}

#[test]
fn healing_backup_then_export_without_cross_signing() {
    let mut app = test_app();
    app.recovery = Some(RecoveryModalState::new());

    app.handle_event(AppEvent::RecoveryHealingProgress(HealingStep::Backup));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::Healing(HealingStep::Backup)
    );

    app.handle_event(AppEvent::RecoveryHealingProgress(
        HealingStep::ExportSecrets,
    ));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::Healing(HealingStep::ExportSecrets)
    );

    app.handle_event(AppEvent::RecoveryKeyReady("newkey123".to_string()));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::ShowKey("newkey123".to_string())
    );
}

#[test]
fn healing_full_path_with_cross_signing() {
    let mut app = test_app();
    app.recovery = Some(RecoveryModalState::new());

    app.handle_event(AppEvent::RecoveryHealingProgress(HealingStep::CrossSigning));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::Healing(HealingStep::CrossSigning)
    );

    let (tx, _rx) = tokio::sync::oneshot::channel();
    app.handle_event(AppEvent::RecoveryNeedPassword(
        crate::event::PasswordSender::new(tx),
    ));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::NeedPassword
    );

    app.handle_event(AppEvent::RecoveryHealingProgress(HealingStep::Backup));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::Healing(HealingStep::Backup)
    );

    app.handle_event(AppEvent::RecoveryHealingProgress(
        HealingStep::ExportSecrets,
    ));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::Healing(HealingStep::ExportSecrets)
    );

    app.handle_event(AppEvent::RecoveryKeyReady("abc".to_string()));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::ShowKey("abc".to_string())
    );
}

#[test]
fn healing_from_resetting_stage() {
    let mut app = test_app();
    app.recovery = Some(RecoveryModalState::new());
    app.recovery.as_mut().unwrap().stage = RecoveryStage::Resetting;

    app.handle_event(AppEvent::RecoveryHealingProgress(HealingStep::Backup));
    assert_eq!(
        app.recovery.as_ref().unwrap().stage,
        RecoveryStage::Healing(HealingStep::Backup)
    );
}
