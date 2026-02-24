# Walrust Architecture

## Overview

Walrust is a Rust-based terminal Matrix client with vim motions and a cyberpunk aesthetic.
It uses ratatui for rendering, crossterm for terminal input, and matrix-sdk for the Matrix protocol.

## Module Structure

```
src/
├── main.rs                  # Tokio main, terminal setup, event loop
├── app.rs                   # App struct (all state), event dispatch
├── event.rs                 # AppEvent enum, event channel setup
├── terminal.rs              # Terminal init/restore, Tui wrapper
├── config.rs                # Data dir paths, constants
│
├── input/
│   ├── mod.rs               # InputResult enum, re-exports
│   ├── vim.rs               # VimMode enum, VimState machine
│   ├── handler.rs           # Top-level key dispatch by mode
│   ├── normal.rs            # Normal mode: hjkl, gg, G, Tab, Enter, /, :
│   ├── insert.rs            # Insert mode: typing, Enter to send, Esc
│   └── command.rs           # Command mode: parse :quit, :join, :leave, :dm
│
├── ui/
│   ├── mod.rs               # Top-level render(app, frame)
│   ├── layout.rs            # Two-panel constraint layout
│   ├── theme.rs             # Cyberpunk color constants
│   ├── room_list.rs         # Left panel: spaces/rooms/DMs list
│   ├── chat.rs              # Right panel: message timeline
│   ├── input_bar.rs         # Message composition / command bar
│   ├── status_bar.rs        # Bottom bar: mode, room name, sync status
│   └── login.rs             # Dedicated login screen
│
├── matrix/
│   ├── mod.rs               # Re-exports
│   ├── client.rs            # Matrix client build, login, restore
│   ├── session.rs           # Session JSON persistence
│   ├── sync.rs              # Sync loop task, event handlers → channel
│   ├── rooms.rs             # Room list extraction from SDK
│   └── messages.rs          # Message fetch, pagination, send
│
└── state/
    ├── mod.rs               # Re-exports
    ├── auth.rs              # AuthState enum
    ├── rooms.rs             # RoomListState, RoomSummary
    └── messages.rs          # MessageState, DisplayMessage, pagination
```

## Async ↔ TUI Bridge

The central design challenge: ratatui rendering is synchronous, Matrix SDK sync is async.
Bridged via `tokio::sync::mpsc::unbounded_channel`:

```
  ┌─────────────────┐     ┌──────────────────┐
  │  Matrix Sync    │     │  Crossterm Input  │
  │  (tokio task)   │     │  (tokio task)     │
  └────────┬────────┘     └────────┬──────────┘
           │ AppEvent              │ AppEvent
           ▼                      ▼
  ┌─────────────────────────────────────────┐
  │     UnboundedReceiver<AppEvent>         │
  └───────────────────┬─────────────────────┘
                      ▼
  ┌─────────────────────────────────────────┐
  │              Main Loop                  │
  │  tokio::select! {                       │
  │    event = rx.recv() => app.handle(ev)  │
  │    _ = tick_interval  => app.tick()     │
  │    _ = render_interval => draw(app)     │
  │  }                                      │
  └─────────────────────────────────────────┘
```

## Event System

All events flow through a single `AppEvent` enum:

- **Key(KeyEvent)** - Keyboard input from crossterm
- **Resize(u16, u16)** - Terminal resize
- **Tick** - Periodic tick for animations/status updates
- **LoginSuccess** - Successful Matrix authentication
- **LoginFailure(String)** - Failed authentication
- **LoggedOut** - User logged out
- **RoomListUpdated(Vec<RoomSummary>)** - Room list changed
- **NewMessage { room_id, message }** - New message received
- **MessageSent { room_id, event_id }** - Message confirmed sent
- **SendError { room_id, error }** - Message send failed
- **MessagesLoaded { room_id, messages, has_more }** - History loaded
- **SyncError(String)** - Sync loop error
- **SyncStatus(String)** - Sync status update

## Vim Modal System

Three modes with clear transitions:

```
         i                     :
Normal ─────► Insert     Normal ─────► Command
  ▲              │         ▲                │
  │    Esc       │         │   Esc/Enter    │
  └──────────────┘         └────────────────┘
```

### Normal Mode
- **j/k** - Move down/up in focused panel
- **h/l** - Collapse/expand spaces or unused
- **Tab** - Switch focus between panels
- **Enter** - Select room or expand space
- **gg** - Jump to top
- **G** - Jump to bottom
- **/** - Start search/filter
- **:** - Enter command mode
- **i** - Enter insert mode (when in chat panel)

### Insert Mode
- Text input for messages
- **Enter** - Send message
- **Esc** - Return to Normal mode

### Command Mode
- **:q** / **:quit** - Quit application
- **:join #room:server** - Join a room
- **:leave** - Leave current room
- **:dm @user:server** - Start direct message
- **:logout** - Log out and clear session
- **Esc** - Cancel and return to Normal
- **Enter** - Execute command

## UI Layout

```
┌──────────┬──────────────────────────────────┐
│ ROOMS    │ > #general                       │
│          │                                  │
│ ≡ Home   │ neo [12:01]                      │
│ ≡ Work   │  hey everyone                    │
│          │                                  │
│ #general │ trinity [12:02]                  │
│ #random  │  welcome back                    │
│ #dev     │                                  │
│          │ morpheus [12:03]                  │
│ DMs      │  the matrix has you              │
│          │                                  │
│          ├──────────────────────────────────┤
│          │ > type message here...            │
├──────────┴──────────────────────────────────┤
│ NORMAL │ #general │ synced                   │
└──────────────────────────────────────────────┘
```

## Cyberpunk Theme

| Element | Color | Hex |
|---------|-------|-----|
| Background | Deep black | `#0a0a0f` |
| Primary accent | Neon cyan | `#00ffff` |
| Secondary accent | Neon magenta | `#ff00ff` |
| Success/Insert | Neon green | `#00ff80` |
| Error | Warm red | `#ff503c` |
| Primary text | Light gray | `#dcdce6` |
| Dimmed text | Dim gray | `#78788c` |
| Borders | Dark gray-blue | `#28323c` |

## Session Persistence

Sessions are stored at `~/.local/share/walrust/session.json` containing:
- Homeserver URL
- User ID
- Device ID
- Access token
- SQLite store path

On startup, if a session file exists, the client attempts to restore the session
and skip the login screen.

## Data Flow

1. User presses key → crossterm event → `AppEvent::Key` → channel
2. Main loop receives event → dispatches to input handler based on vim mode
3. Input handler returns `InputResult` (action to take)
4. App processes action (e.g., select room, send message, quit)
5. For Matrix operations: spawn async task → result comes back as AppEvent
6. State updates trigger re-render on next render tick
