```
 ██████╗  ██████╗ ███████╗██╗   ██╗████████╗ ██████╗
██╔════╝ ██╔═══██╗██╔════╝██║   ██║╚══██╔══╝██╔═══██╗
██║  ███╗██║   ██║███████╗██║   ██║   ██║   ██║   ██║
██║   ██║██║   ██║╚════██║██║   ██║   ██║   ██║   ██║
╚██████╔╝╚██████╔╝███████║╚██████╔╝   ██║   ╚██████╔╝
 ╚═════╝  ╚═════╝ ╚══════╝ ╚═════╝    ╚═╝    ╚═════╝
```

**Gōsuto** (ゴースト) — _ghost_ — a cyberpunk terminal Matrix client. Efficient by design: vim motions for navigation, <60MB of RAM for everything.

https://github.com/user-attachments/assets/c58be922-67d6-400c-aebc-69db3c62a24f

## ═══ why

I switched from Discord to Matrix and couldn't find a native terminal client that did voice calls with push-to-talk. Element is Electron — 800MB of RAM to sit idle. I spend most of my day in a terminal anyway. So I built what I actually wanted: a single Rust binary that handles chat, E2EE, and voice without a browser engine underneath.

## ═══ what it does

- Vim-first navigation — Normal, Insert, Command modes
- Encrypted chat — rooms, spaces, DMs, full E2EE with automatic key forwarding
- VoIP calls — LiveKit-based voice with push-to-talk support
- Room management — create, join, leave, view member lists and power levels
- Visual effects — matrix rain, glitch, text reveal animations (all togglable)

## ═══ install

Pre-built binaries for **Linux** and **Windows** are available on the [releases page](https://github.com/maikbuse/gosuto/releases).

### Linux

Download the binary, make it executable, and move it somewhere on your `PATH`:

```bash
chmod +x gosuto
sudo mv gosuto /usr/local/bin/
```

### Windows

Download `gosuto.exe` and place it in a directory on your `PATH`, or run it directly:

```powershell
.\gosuto.exe
```

### Install from crates.io

```bash
cargo install gosuto
```

### Build from source

```bash
# clone and build
git clone https://github.com/maikbuse/gosuto.git
cd gosuto
cargo build --release
./target/release/gosuto        # Linux
.\target\release\gosuto.exe    # Windows
```

Run `gosuto` and log in with your Matrix homeserver, username, and password.

## ═══ supported terminals

Gosuto works on any modern terminal emulator — Kitty, WezTerm, Ghostty, Foot, Alacritty, GNOME Terminal, Windows Terminal, and others. Terminal multiplexers (tmux, screen) are also supported.

## ═══ optional enhancements

- **Nerd Font** — enables icon glyphs throughout the UI. Toggleable in config; falls back to plain Unicode when disabled.

## ═══ finding your way around

Gosuto has a which-key popup — press a key (e.g. the spacebar) in normal mode and it shows you what's available. Command mode (`:`) has tab completion and suggestions. Between those two, you shouldn't need to memorize anything from the docs.

## ═══ config

Configuration is stored in `config.toml` inside the platform config directory:

| Platform | Path |
|----------|------|
| Linux    | `~/.config/gosuto/config.toml` |
| Windows  | `%APPDATA%\gosuto\config.toml` |

A default config file is created on first launch. Edit it to change network, audio, UI, and visual effect settings.

## ═══ data

Session and runtime data live in the platform data directory:

| Platform | Path |
|----------|------|
| Linux    | `~/.local/share/gosuto/` |
| Windows  | `%LOCALAPPDATA%\gosuto\` |

```
 session.json   encrypted session credentials
 store/         matrix-sdk SQLite store
 logs/          log files
```

To enable logging, set the `GOSUTO_LOG` environment variable before launching:

```bash
GOSUTO_LOG=debug gosuto                        # Linux
```

```powershell
$env:GOSUTO_LOG="debug"; .\gosuto.exe          # Windows (PowerShell)
```

## ═══ limitations

- **E2EE calls with Element X** — Voice calls between Gosuto and Element X are not end-to-end encrypted. Element X uses a JavaScript-based encryption implementation (HKDF key derivation) that differs from the native C++ implementation (PBKDF2) used by Gosuto. Calls between Gosuto users work with full E2EE.

## ═══ license

Licensed under either of

- [Apache License, Version 2.0](LICENSE.md#apache-license-version-20)
- [MIT License](LICENSE.md#mit-license)

at your option.
