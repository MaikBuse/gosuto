# Technical Debt Audit

Comprehensive audit of the Gosuto codebase conducted 2026-03-09.

---

## 1. ~~God Module: `app.rs` (2,991 lines)~~ — RESOLVED

State structs extracted into `src/state/` modules with `handle_key() -> Action`
pattern. `app.rs` then split into `src/app/` directory module (296-line `mod.rs`
with 6 submodules: `event_handler`, `input_handler`, `commands`, `modal_keys`,
`audio`, `tests`).

### Checklist

- [x] Extract `RoomInfoState` into `src/state/room_info.rs`
- [x] Extract `CreateRoomState` into `src/state/create_room.rs`
- [x] Extract `AudioSettingsState` into `src/state/audio_settings.rs`
- [x] Extract `UserConfigState` into `src/state/user_config.rs`
- [x] Extract `ChangePasswordState` into `src/state/change_password.rs`
- [x] Extract `RecoveryModalState` into `src/state/recovery.rs`
- [x] Move per-modal key handlers into their respective state modules
- [x] Split `app.rs` into `src/app/` directory module — `mod.rs` at 296 lines

---

## 2. ~~Mutex Panic Risk: 10 `lock().unwrap()` calls~~ — RESOLVED

Replaced all `std::sync::Mutex` with `parking_lot::Mutex` (and
`parking_lot::RwLock` for the read-only `audio_config`). All `.unwrap()`
calls on mutex locks removed — `parking_lot::Mutex::lock()` returns the
guard directly without poisoning.

### Checklist

- [x] Add `parking_lot` to `Cargo.toml`
- [x] Replace `std::sync::Mutex` with `parking_lot::Mutex` in `src/app/`
- [x] Replace `std::sync::Mutex` with `parking_lot::Mutex` in `src/global_ptt.rs`
- [x] Replace `std::sync::Mutex` with `parking_lot::Mutex` in `src/event.rs`
- [x] Replace `std::sync::Mutex` with `parking_lot::Mutex` in `src/voip/audio.rs`
- [x] Replace `std::sync::Mutex` with `parking_lot::RwLock` in `src/voip/manager.rs`
- [x] Remove all `.unwrap()` calls on mutex locks (parking_lot returns value directly)

---

## 3. ~~Silent Error Swallowing: 179 `let _ = ...` instances~~ — RESOLVED

Added `WarnClosed` extension trait in `src/event.rs` to log channel-closed errors.
~150 channel `.send()` calls now use `.warn_closed("VariantName")`. ~25 non-channel
patterns (keyring, SDK ops, oneshot sends) got individual `if let Err(e)` treatment.
~7 intentionally fire-and-forget instances left as-is (panic hook, app exit, log
cleanup, terminal degradation, test helper).

### Checklist

- [x] Audit `src/main.rs` — classify each as ignorable / should-log / should-propagate
- [x] Audit `src/voip/manager.rs` — same classification
- [x] Audit `src/matrix/verification.rs` — same classification
- [x] Audit `src/matrix/rooms.rs` — same classification
- [x] Audit `src/matrix/sync.rs` — same classification
- [x] Audit remaining 11 files
- [x] Replace should-log instances with `WarnClosed` trait / `if let Err(e)` logging
- [x] Replace should-propagate instances with proper `?` propagation

---

## 4. ~~Silent Data Loss in VoIP via `unwrap_or_default()`~~ — RESOLVED

Replaced silent `unwrap_or_default()` calls with logged warnings/errors in VoIP
protocol parsing and audio resampling. Remaining `unwrap_or_default()` calls
reviewed and confirmed acceptable (system clock, debug serialization, device
enumeration, Option field defaults).

### Checklist

- [x] Replace `resp.json().await.unwrap_or_default()` with proper error propagation (matrixrtc.rs:45)
- [x] Replace `resp.text().await.unwrap_or_default()` with proper error propagation (matrixrtc.rs:195)
- [x] Replace `serde_json::from_str().unwrap_or_default()` with logged error (matrixrtc.rs:209)
- [x] Add `warn!` logging for resampler pop defaults (audio.rs:500, 693)
- [x] Review remaining `unwrap_or_default()` calls for appropriateness

---

## 5. Missing Test Coverage for Critical Modules

277 tests exist but are concentrated in `app/tests.rs` (integration) and
`state/` (recovery state machine, rooms, members, messages). The most complex
and bug-prone modules have zero tests.

**Untested critical modules:**
- `src/voip/` — call establishment, audio pipeline, MatrixRTC protocol (2,283 lines)
- `src/matrix/sync.rs` — room sync, event handling, error recovery
- `src/matrix/client.rs` — session restore, reconnection
- `src/input/command.rs` — command parsing and completion (708 lines of pure logic)

### Checklist

- [x] Add unit tests for `src/input/command.rs` — already has 50+ tests
- [x] Add unit tests for `src/input/vim.rs` — already has 50+ tests
- [ ] Add state machine tests for `src/voip/manager.rs` — async methods need trait-based DI (separate PR)
- [x] Add tests for `src/voip/matrixrtc.rs` — pure function tests (parse_livekit_identity, lenient_base64_decode)
- [x] Add tests for `src/matrix/sync.rs` — pure function + dispatch_encryption_keys tests
- [ ] Add tests for `src/matrix/client.rs` — only `normalize_homeserver_url` testable (already has 8 tests)

---

## Other Notable Issues

| Issue | Location | Severity |
|-------|----------|----------|
| `main.rs` at 1,065 lines with mixed concerns | `src/main.rs` | Medium |
| `thread::spawn().expect()` panics if PTT thread fails | `src/global_ptt.rs:148` | Medium |
| `unreachable!()` in recovery action match | `src/main.rs:811` | Low |
| Fire-and-forget `tokio::spawn` without tracking JoinHandles | `src/main.rs`, `src/matrix/` | Medium |
| Audio tasks killed via `.abort()` without graceful shutdown | `src/voip/audio.rs` | Medium |
| ~~`Mutex` used where `RwLock` fits better (read-heavy `audio_config`)~~ | ~~`src/voip/manager.rs`~~ | ~~DONE~~ |

---

## Recommended Priority Order

1. ~~**Mutex panic risk** — smallest change, highest reliability impact~~ DONE
2. ~~**VoIP `unwrap_or_default()`** — prevents silent call failures~~ DONE
3. ~~**Audit `let _ =` patterns** — systematic pass, log or propagate~~ DONE
4. **Add tests for `input/command.rs`** — pure logic, highest ROI
5. ~~**Split `app.rs`** — largest structural improvement~~ DONE
