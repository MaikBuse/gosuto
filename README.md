```
 ██████╗  ██████╗ ███████╗██╗   ██╗████████╗ ██████╗
██╔════╝ ██╔═══██╗██╔════╝██║   ██║╚══██╔══╝██╔═══██╗
██║  ███╗██║   ██║███████╗██║   ██║   ██║   ██║   ██║
██║   ██║██║   ██║╚════██║██║   ██║   ██║   ██║   ██║
╚██████╔╝╚██████╔╝███████║╚██████╔╝   ██║   ╚██████╔╝
 ╚═════╝  ╚═════╝ ╚══════╝ ╚═════╝    ╚═╝    ╚═════╝
```

**Gōsuto** (ゴースト) — _ghost_ — a cyberpunk terminal Matrix client with vim motions.

## ═════════ why

I switched from Discord to Matrix and couldn't find a native terminal client that did voice calls with push-to-talk. Element is Electron — 800MB of RAM to sit idle. I spend most of my day in a terminal anyway. So I built what I actually wanted: a single Rust binary that handles chat, E2EE, and voice without a browser engine underneath.

## ═════════ what it does

- Vim-first navigation — Normal, Insert, Command modes
- Encrypted chat — rooms, spaces, DMs, full E2EE with automatic key forwarding
- VoIP calls — LiveKit-based voice with push-to-talk support
- Room management — create, join, leave, view member lists and power levels
- Visual effects — matrix rain, glitch, text reveal animations (all togglable)

## ═════════ install

```bash
# install directly
cargo install --git https://github.com/maikbuse/gosuto.git

# or clone and build
git clone https://github.com/maikbuse/gosuto.git
cd gosuto
cargo build --release
./target/release/gosuto
```

Run `gosuto` and log in with your Matrix homeserver, username, and password.

## ═════════ finding your way around

Gosuto has a which-key popup — press a key in normal mode and it shows you what's available. Command mode (`:`) has tab completion and suggestions. Between those two, you shouldn't need to memorize anything from a README.

## ═════════ config

Data lives in `~/.local/share/gosuto/`:

```
 session.json   encrypted session credentials
 store/         matrix-sdk SQLite store
 logs/          log files (enable with GOSUTO_LOG=debug gosuto)
```

## ═════════ license

Licensed under either of

- [Apache License, Version 2.0](LICENSE.md#apache-license-version-20)
- [MIT License](LICENSE.md#mit-license)

at your option.
