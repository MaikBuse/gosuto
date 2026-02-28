```
 ██████╗  ██████╗ ███████╗██╗   ██╗████████╗ ██████╗
██╔════╝ ██╔═══██╗██╔════╝██║   ██║╚══██╔══╝██╔═══██╗
██║  ███╗██║   ██║███████╗██║   ██║   ██║   ██║   ██║
██║   ██║██║   ██║╚════██║██║   ██║   ██║   ██║   ██║
╚██████╔╝╚██████╔╝███████║╚██████╔╝   ██║   ╚██████╔╝
 ╚═════╝  ╚═════╝ ╚══════╝ ╚═════╝    ╚═╝    ╚═════╝
```

**Gōsuto** (ゴースト) — _ghost_ — a cyberpunk terminal Matrix client with vim motions.

```
#00ffff cyan │ #ff00ff magenta │ #00ff80 green │ #ff503c red │ #0a0a0f black
───────────────────────────────────────────────────────────────────────────────
  focus/normal │   command mode  │  insert mode  │   errors    │  background
```

## ════════════════════ the point ════════════════════

~3,400 lines of Rust. One static binary. No Electron, no browser, no bloat.

- Vim-first — Normal, Insert, Command modes. If you know vim, you know Gosuto.
- Full E2EE via matrix-sdk with automatic room key forwarding
- Async Tokio runtime, 50ms render cycle
- Neon-on-black palette, matrix rain, glitch effects, text reveal animations

## ════════════════════ features ════════════════════

### chat

Browse and join rooms, spaces, and DMs. Send encrypted messages. Scroll history with `j`/`k`. Date separators between days.

Power levels: `~` owner `&` admin `@` op `+` voice
Room glyphs: `≡` spaces `#` rooms `@` DMs

Room creation with configurable history visibility.

### voip

LiveKit-based voice calls. Configure audio devices with `:audio`. Start, answer, reject, hangup.

### effects

- **matrix rain** — cascading green characters across the terminal
- **glitch** — randomized text corruption
- **text reveal** — characters materialize on the login screen

## ════════════════════ keybindings ════════════════════

Three modes, color-coded in the status bar:

```
 Normal  │ cyan    │ Esc from Insert/Command
 Insert  │ green   │ i
 Command │ magenta │ :
```

### normal mode

```
 j / ↓       move down             h          focus left panel
 k / ↑       move up               l          focus right panel
 gg          jump to top            Tab        cycle panel focus
 G           jump to bottom         Enter      select item
 /           search / filter        i          insert mode
 :           command mode           q          quit
 c           call member            a          answer call
 r           reject call
```

### insert mode

Type your message. `Enter` sends. `Esc` returns to Normal.

## ════════════════════ commands ════════════════════

```
 :quit, :q                     exit gosuto
 :join <room>                  join a room
 :leave                        leave current room
 :dm <user>                    direct message a user
 :create <name> [visibility]   create a room        (alias: :new)
 :info                         show room info       (alias: :roominfo)
 :call                         start a call
 :answer                       answer a call        (alias: :accept)
 :reject                       reject a call        (alias: :decline)
 :hangup                       end a call           (alias: :end)
 :audio                        audio config         (alias: :sound)
 :rain                         toggle matrix rain   (alias: :matrix, :effects)
 :glitch                       toggle glitch effect
 :logout                       log out
```

## ════════════════════ install ════════════════════

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

## ════════════════════ config ════════════════════

Data lives in `~/.local/share/gosuto/`:

```
 session.json   encrypted session credentials
 store/         matrix-sdk SQLite store
 logs/          log files (enable with GOSUTO_LOG=debug gosuto)
```

## ════════════════════ license ════════════════════

Licensed under either of

- [Apache License, Version 2.0](LICENSE.md#apache-license-version-20)
- [MIT License](LICENSE.md#mit-license)

at your option.
