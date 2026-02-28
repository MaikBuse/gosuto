# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo check          # Type-check without building
cargo build          # Debug build
cargo run            # Run the TUI client
GOSUTO_LOG=debug cargo run  # Run with file-based debug logging
```

No tests, linting config, or CI are configured. Use `cargo clippy` and `cargo fmt` with defaults.

## Design

### Cyberpunk Aesthetic

- Neon-on-black palette: cyan (`#00ffff`) primary, magenta (`#ff00ff`) secondary, green (`#00ff80`) success/insert, red (`#ff503c`) errors, on deep black (`#0a0a0f`)
- Semantic color language: cyan = focus/active, magenta = command mode, green = insert mode, red = errors
- IRC heritage: power level prefixes (`~` owner, `&` admin, `@` op, `+` voice), room glyphs (`≡` spaces, `#` rooms, `@` DMs)
- Focus feedback: active panel gets cyan border, inactive panels get dim borders
- Rotating sender colors for chat message distinction

### Vim-First Interaction Model

- Three modes: Normal (navigation), Insert (composition), Command (`:` actions)
- Every action has a keyboard binding — no mouse interaction (mouse enabled at OS level for terminal text selection only)
- Panel focus via `h`/`l`/`Tab` across the 3-column layout (RoomList | Messages | Members)
- Navigation via `j`/`k`, `gg`/`G` within focused panel
- `:commands` for meta-actions (`:join`, `:leave`, `:dm`, `:call`, `:q`, `:logout`)
- `/` for room search/filter
- Status bar communicates mode with color-coded indicator (cyan=Normal, green=Insert, magenta=Command)
- Input bar shows contextual hints: "press i to type, : for commands" in Normal mode
- The UI remains discoverable for non-vim users through visual hierarchy, panel borders, and hints

## Architecture

Gōsuto (ゴースト) is a terminal Matrix chat client (~3,400 LOC Rust) with vim motions and a cyberpunk aesthetic, built on ratatui + crossterm + matrix-sdk. See `docs/architecture.md` for detailed diagrams and data flows.

### Core Pattern: Async-TUI Event Bridge

All async operations (Matrix sync, keyboard input, tick timer, VoIP) send `AppEvent` variants through a single `tokio::sync::mpsc::unbounded_channel`. The main loop in `main.rs` uses `tokio::select!` to receive events, dispatch to `app.handle_event()`, and render on a 50ms interval.

Actions that require async work (sending messages, joining rooms) are queued as pending actions on `App`, then picked up by the main loop which spawns the appropriate tokio task. Results flow back as `AppEvent` variants.

### Module Responsibilities

- **`main.rs`** — Tokio runtime, terminal setup/restore, event loop, spawns async tasks from pending actions
- **`app.rs`** — `App` struct holding all state; `handle_event()` processes events, `process_input()` translates `InputResult` to state changes
- **`event.rs`** — `AppEvent` enum definition and channel type aliases
- **`input/`** — Vim modal system: `VimState` tracks mode (Normal/Insert/Command) and focus panel; `handler.rs` dispatches to mode-specific handlers that return `InputResult` enums
- **`ui/`** — Stateless rendering functions that take `&App` and draw to `Frame`. Layout is a 3-column design (room list | chat+input | members) with status bar at bottom
- **`matrix/`** — Matrix SDK integration: login/session restore (`client.rs`), sync loop with event handlers (`sync.rs`), message fetch/send (`messages.rs`), room list extraction (`rooms.rs`)
- **`state/`** — Domain state types: `AuthState`, `RoomListState`, `MessageState`, `MemberListState`
- **`voip/`** — WebRTC VoIP: `CallManager` actor pattern with command channel, audio pipeline (cpal + audiopus), SDP/ICE signaling
- **`config.rs`** — Path constants: data dir `~/.local/share/gosuto/`, session file, sqlite store, logs

### Key Conventions

- Rust edition 2024
- Error handling: `anyhow::Result` for application errors, `thiserror` for typed errors
- Matrix client shared across tasks via `Arc<Mutex<Option<Client>>>`
- UI rendering is purely functional — `render(app, frame)` reads state, never mutates it
- Messages use inverted scroll (offset 0 = newest at bottom)
- Logging goes to files in `~/.local/share/gosuto/logs/`, controlled by `GOSUTO_LOG` env var
