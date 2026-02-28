```
 ██████╗  ██████╗ ███████╗██╗   ██╗████████╗ ██████╗
██╔════╝ ██╔═══██╗██╔════╝██║   ██║╚══██╔══╝██╔═══██╗
██║  ███╗██║   ██║███████╗██║   ██║   ██║   ██║   ██║
██║   ██║██║   ██║╚════██║██║   ██║   ██║   ██║   ██║
╚██████╔╝╚██████╔╝███████║╚██████╔╝   ██║   ╚██████╔╝
 ╚═════╝  ╚═════╝ ╚══════╝ ╚═════╝    ╚═╝    ╚═════╝
```

**Gōsuto** (ゴースト) — _ghost_ — a cyberpunk terminal Matrix client with vim motions.

<!-- TODO: Add demo video -->

## Table of Contents

- [Why Gosuto](#why-gosuto)
- [Features](#features)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [License](#license)

## Why Gosuto

- **Lightweight** — ~3,400 LOC Rust, compiles to a single static binary
- **Performant** — async Tokio runtime with a 50ms render cycle
- **Vim-first** — Normal, Insert, and Command modes with familiar keybindings
- **Cyberpunk aesthetic** — neon-on-black palette, matrix rain, glitch effects, text reveal animations
- **End-to-end encrypted** — full E2EE via matrix-sdk with automatic room key forwarding

## Features

### Chat

- Browse and join rooms, spaces, and DMs
- Send and receive messages with full E2E encryption
- Scroll through message history with inverted scroll (newest at bottom)
- Date separators between messages from different days
- Room creation with configurable history visibility
- Power level prefixes: `~` owner, `&` admin, `@` op, `+` voice
- Room glyphs: `≡` spaces, `#` rooms, `@` DMs

### VoIP

- LiveKit-based voice calls
- Audio device configuration (`:audio`)
- Call controls: start, answer, reject, hangup

### Visual Effects

- **Matrix rain** — cascading green characters across the terminal
- **Glitch** — randomized text corruption effect
- **Text reveal** — characters materialize progressively on the login screen

### Keybindings

Gosuto uses three vim-inspired modes:

| Mode | Indicator | Enter | Exit |
|------|-----------|-------|------|
| **Normal** | Cyan | `Esc` from Insert/Command | — |
| **Insert** | Green | `i` | `Esc` |
| **Command** | Magenta | `:` | `Esc` or `Enter` |

**Normal mode:**

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `gg` | Jump to top |
| `G` | Jump to bottom |
| `h` | Focus left panel |
| `l` | Focus right panel |
| `Tab` | Cycle panel focus |
| `Enter` | Select item |
| `/` | Search / filter rooms |
| `i` | Enter Insert mode |
| `:` | Enter Command mode |
| `c` | Call selected member |
| `a` | Answer incoming call |
| `r` | Reject incoming call |
| `q` | Quit |

**Insert mode:** Type your message, press `Enter` to send, `Esc` to return to Normal.

### Commands

| Command | Aliases | Description |
|---------|---------|-------------|
| `:quit` | `:q` | Exit gosuto |
| `:join <room>` | | Join a room |
| `:leave` | | Leave current room |
| `:dm <user>` | | Direct message a user |
| `:create <name> [visibility]` | `:new` | Create a new room |
| `:info` | `:roominfo` | Show room info |
| `:call` | | Start a call in current room |
| `:answer` | `:accept` | Answer incoming call |
| `:reject` | `:decline` | Reject incoming call |
| `:hangup` | `:end` | End active call |
| `:audio` | `:sound` | Audio device configuration |
| `:rain` | `:matrix`, `:effects` | Toggle matrix rain effect |
| `:glitch` | | Toggle glitch effect |
| `:logout` | | Log out of session |

## Quick Start

### Build from source

```bash
# Install directly
cargo install --git https://github.com/maikbuse/gosuto.git

# Or clone and build
git clone https://github.com/maikbuse/gosuto.git
cd gosuto
cargo build --release
./target/release/gosuto
```

### Run

```bash
gosuto
```

You'll be greeted with the login screen. Enter your Matrix homeserver, username, and password.

## Configuration

Gosuto stores its data in `~/.local/share/gosuto/`:

| Path | Purpose |
|------|---------|
| `session.json` | Encrypted session credentials |
| `store/` | matrix-sdk SQLite store |
| `logs/` | Log files |

### Logging

Enable debug logging with the `GOSUTO_LOG` environment variable:

```bash
GOSUTO_LOG=debug gosuto
```

Logs are written to `~/.local/share/gosuto/logs/`.

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE.md#apache-license-version-20)
- [MIT License](LICENSE.md#mit-license)

at your option.
