<a id="readme-top"></a>

<!-- PROJECT SHIELDS -->
[![License][license-shield]][license-url]
[![Crates.io][crates-shield]][crates-url]
[![Stars][stars-shield]][stars-url]

<!-- PROJECT LOGO -->
<br />
<div align="center">
  <a href="https://github.com/MaikBuse/gosuto">
    <img src="assets/logo.svg" alt="Logo" width="240">
  </a>

  <h3 align="center">Gōsuto</h3>

  <p align="center">
    ゴースト — <em>ghost</em> — a cyberpunk terminal Matrix client
    <br />
    <a href="https://github.com/MaikBuse/gosuto/releases">Releases</a>
    &middot;
    <a href="https://github.com/MaikBuse/gosuto/issues">Report Bug</a>
    &middot;
    <a href="https://github.com/MaikBuse/gosuto/issues">Request Feature</a>
  </p>
</div>

<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#built-with">Built With</a></li>
      </ul>
    </li>
    <li><a href="#features">Features</a></li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#installation">Installation</a></li>
        <li><a href="#supported-terminals">Supported Terminals</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#voip--prebuilt-libwebrtc">VoIP & Prebuilt libwebrtc</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
  </ol>
</details>

<!-- ABOUT THE PROJECT -->
## About The Project

<https://github.com/user-attachments/assets/c58be922-67d6-400c-aebc-69db3c62a24f>

Gōsuto is a native terminal client for the [Matrix](https://matrix.org/) protocol, built for people who live in the terminal. It ships as a single Rust binary that handles chat, end-to-end encryption, and voice calls — no browser engine, no heavy runtime, just a lightweight TUI that stays out of your way.

The goal is simple: a fast, keyboard-driven Matrix experience with full voice support and under 60 MB of RAM.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Built With

* [Rust](https://www.rust-lang.org/)
* [Ratatui](https://ratatui.rs/)
* [matrix-sdk](https://github.com/matrix-org/matrix-rust-sdk)
* [LiveKit Rust SDK](https://github.com/livekit/rust-sdks)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- FEATURES -->
## Features

* **Vim-first navigation** — Normal, Insert, and Command modes
* **Encrypted chat** — rooms, spaces, DMs, full E2EE with automatic key forwarding
* **VoIP calls** — LiveKit-based voice with push-to-talk support
* **Room management** — create, join, leave, view member lists and power levels
* **Visual effects** — matrix rain, glitch, text reveal animations (all togglable)
* **Element compatible** — tested against Element Web and Element X
* **Lightweight** — under 60 MB of RAM for everything

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- GETTING STARTED -->
## Getting Started

### Prerequisites

* **Rust toolchain** (only if building from source) — install via [rustup](https://rustup.rs/)
* **Nerd Font** (optional) — enables icon glyphs throughout the UI; toggleable in config, falls back to plain Unicode when disabled

### Installation

#### Pre-built binaries

Pre-built binaries for **Linux** and **Windows** are available on the [releases page](https://github.com/MaikBuse/gosuto/releases).

**Linux:**

```bash
chmod +x gosuto
sudo mv gosuto /usr/local/bin/
```

**Windows:**

Download `gosuto.exe` and place it in a directory on your `PATH`, or run it directly:

```powershell
.\gosuto.exe
```

#### Install from crates.io

```bash
cargo install gosuto
```

#### Build from source

```bash
git clone https://github.com/MaikBuse/gosuto.git
cd gosuto
cargo build --release
./target/release/gosuto        # Linux
.\target\release\gosuto.exe    # Windows
```

Run `gōsuto` and log in with your Matrix homeserver, username, and password.

### Supported Terminals

Gōsuto works on any modern terminal emulator — Kitty, WezTerm, Ghostty, Foot, Alacritty, GNOME Terminal, Windows Terminal, and others. Terminal multiplexers (tmux, screen) are also supported.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- USAGE -->
## Usage

Gōsuto has a **which-key popup** — press a key (e.g. the spacebar) in normal mode and it shows you what's available. Command mode (`:`) has tab completion and suggestions. Between those two, you shouldn't need to memorize anything.

### Config

Configuration is stored in `config.toml` inside the platform config directory:

| Platform | Path |
|----------|------|
| Linux    | `~/.config/gosuto/config.toml` |
| Windows  | `%APPDATA%\gosuto\config.toml` |

A default config file is created on first launch. Edit it to change network, audio, UI, and visual effect settings.

### Data & Logging

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

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- VOIP -->
## VoIP & Prebuilt libwebrtc

Gōsuto uses a [fork](https://github.com/MaikBuse/gosuto-livekit-sdks) of the [LiveKit Rust SDK](https://github.com/livekit/rust-sdks) for voice calls. The fork adds configurable key derivation (HKDF) so E2EE calls interoperate with Element X, and points the build script at prebuilt libwebrtc m137 binaries hosted as GitHub release assets on the fork repo.

The prebuilt `libwebrtc.a` (Linux) and `webrtc.lib` (Windows) are compiled from the [webrtc-sdk/webrtc m137_release branch](https://github.com/webrtc-sdk/webrtc/tree/m137_release) using the build scripts and patches checked into the fork. If you prefer to verify the native code yourself, you can build libwebrtc from source and point your build at it:

```bash
# Set LK_CUSTOM_WEBRTC to skip the prebuilt download
export LK_CUSTOM_WEBRTC=/path/to/your/libwebrtc/build
cargo build --release
```

See the [build scripts](https://github.com/MaikBuse/gosuto-livekit-sdks/tree/main/webrtc-sys/libwebrtc) in the fork repo for full instructions.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- LICENSE -->
## License

Licensed under either of

* [Apache License, Version 2.0](LICENSE.md#apache-license-version-20)
* [MIT License](LICENSE.md#mit-license)

at your option.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- CONTACT -->
## Contact

Maik Buse — [Homepage](https://buse.io)

Project Link: [https://github.com/MaikBuse/gosuto](https://github.com/MaikBuse/gosuto)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- MARKDOWN LINKS & IMAGES -->
[license-shield]: https://img.shields.io/badge/license-Apache--2.0%20OR%20MIT-blue.svg?style=for-the-badge
[license-url]: https://github.com/MaikBuse/gosuto/blob/main/LICENSE.md
[crates-shield]: https://img.shields.io/crates/v/gosuto.svg?style=for-the-badge
[crates-url]: https://crates.io/crates/gosuto
[stars-shield]: https://img.shields.io/github/stars/MaikBuse/gosuto.svg?style=for-the-badge
[stars-url]: https://github.com/MaikBuse/gosuto/stargazers
