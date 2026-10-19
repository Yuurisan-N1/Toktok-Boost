<div align="center">

<img width="100%" alt="header" src="https://capsule-render.vercel.app/api?type=waving&height=210&text=Toktok%20Boost&fontAlign=50&fontAlignY=36&fontSize=56&desc=TikTok%20Boost%20Tool%20%7C%20Followers%20%7C%20Hearts%20%7C%20Views%20%7C%20Comments%20%7C%20Live&descAlign=50&descAlignY=58"/>

<img alt="typing" src="https://readme-typing-svg.demolab.com?font=Inter&size=18&duration=3000&pause=650&center=true&vCenter=true&width=900&lines=Auto+Captcha+Gate+%7C+Built-in+Glyph+Solver;Follower+Boost+%7C+Profile+%26+Video+Links;Hearts+%2F+Comment+Hearts+%2F+Views+%2F+Shares+%2F+Favorites;Live+Stream+Boost+%7C+Repost+Boost;Searchable+Comment+Picker+%7C+Filter+by+Author+or+Text"/>

<p>
  <img alt="rust" src="https://img.shields.io/badge/Rust-2021-f74c00?logo=rust&logoColor=white"/>
  <img alt="platform" src="https://img.shields.io/badge/Platform-TikTok%20Boost-111111"/>
  <img alt="build" src="https://github.com/Yuurisan-N1/Toktok-Boost/actions/workflows/build.yml/badge.svg"/>
  <img alt="author" src="https://img.shields.io/badge/by-Yuurisandesu-111111"/>
</p>

<p>
  <b>Toktok Boost</b> is an interactive TikTok boost tool built in Rust.<br/>
  It clears the captcha gate automatically using a built-in glyph solver, reads the live service list from the site, and presents an interactive menu. Pick a service, answer the prompts, and the bot submits the boost to the link in <code>data.txt</code>. Cooldowns are handled with automatic countdown and session recovery. Eight services are supported: Followers, Hearts, Comment Hearts, Views, Shares, Favorites, Live Stream, and Repost.<br/>
  Built and distributed by <b>Yuurisandesu</b>.
</p>

</div>

<hr/>

<p align="center">
  <img src="image/image.png" alt="preview" width="100%"/>
</p>

<hr/>

---

## Table of Contents

- [Requirements](#requirements)
- [Installation](#installation)
- [Configuration](#configuration)
- [Running the Bot](#running-the-bot)
- [Download Prebuilt Binary](#download-prebuilt-binary)
- [Features](#features)
- [File Structure](#file-structure)
- [Disclaimer](#disclaimer)

---

## Requirements

- Rust `1.70+` (includes `cargo`, only needed if building from source)
- Ruby `3.0+` (only needed to run the downloader script)
- Git

---

## Install Rust

### Linux / macOS

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

### Windows

Download and run the installer from https://rustup.rs, then restart your terminal. Verify with:

```cmd
rustc --version
```

### Termux (Android)

```bash
pkg update && pkg install proot-distro
proot-distro install ubuntu
proot-distro login ubuntu
```

Then inside Ubuntu:

```bash
apt update && apt install -y curl git build-essential pkg-config libssl-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

---

## Install Ruby

Ruby is only needed to run the downloader script. Skip this section if you download the binary manually.

### Linux

```bash
sudo apt update && sudo apt install -y ruby git
```

### macOS

```bash
brew install ruby
```

### Windows

Download and run the installer from https://rubyinstaller.org. Check **Add Ruby executables to your PATH** during installation.

### Termux (Android)

```bash
pkg update && pkg install ruby git
```

---

## Installation

**Clone the repository:**

```bash
git clone https://github.com/Yuurisan-N1/Toktok-Boost.git
cd Toktok-Boost
```

---

## Configuration

### 1. Target Link (data.txt)

Put one TikTok link in `data.txt`. A profile link, a video link, or a share link all work. Only one link is used. For example:

```
https://www.tiktok.com/@username
```

### 2. Proxy (proxy.txt)

Fill `proxy.txt` with one proxy (optional, leave empty to run without proxy):

```
host:port
host:port:user:pass
http://user:pass@host:port
socks5://user:pass@host:port
```

### 3. Bot Settings (config.json)

`sleep_seconds` controls the pause between cycles. The minimum is 30 seconds. If `config.json` is missing, it is created automatically.

---

## Running the Bot

### Build from Source

```bash
cargo build --release
```

Then run:

```bash
# Linux / macOS / Termux
./target/release/toktok-boost

# Windows
.\target\release\toktok-boost.exe
```

### Using run.sh (Linux / macOS / Termux)

Make it executable first:

```bash
chmod +x run.sh
```

Then choose a mode:

```bash
./run.sh direct    # build and run in foreground (default)
./run.sh nohup     # build and run in background, logs saved to toktok-boost.log
./run.sh screen    # build and run in a detached screen session
./run.sh tmux      # build and run in a detached tmux session
./run.sh logs      # tail the log file
./run.sh stop      # stop the running bot process
```

Attach to a background session:

```bash
# screen
screen -r toktok-boost

# tmux
tmux attach -t toktok-boost
```

### Using make (Linux / macOS)

```bash
make release    # optimized release build
make start      # build release and run immediately
make run        # build debug and run
make check      # check for errors without building
make fmt        # format the code
make clean      # remove all build artifacts
make size       # show release binary size
```

Press `Ctrl+C` at any time to stop the bot cleanly.

---

## Download Prebuilt Binary

### Option 1 - Downloader Script

```bash
ruby bot.rb
```

The script shows a numbered menu with the available platforms. Enter the number for your platform. The binary downloads with a live progress bar and is set to executable automatically on Linux and macOS.

### Option 2 - Manual Download

Download the latest release from:
https://github.com/Yuurisan-N1/Toktok-Boost/releases/latest

| File | Platform |
|---|---|
| `toktok-boost-linux-x86_64` | Linux x86_64 |
| `toktok-boost-macos-aarch64` | macOS Apple Silicon |
| `toktok-boost-windows-x86_64.exe` | Windows x86_64 |

**Linux / macOS after downloading:**

```bash
chmod +x toktok-boost-linux-x86_64
./toktok-boost-linux-x86_64
```

**Windows:**

```bash
.\toktok-boost-windows-x86_64.exe
```

Place the binary in the same folder as your `data.txt`, `proxy.txt`, and `config.json` before running.

---

## Features

### Captcha Gate
Before the service list is shown, the bot clears a captcha challenge automatically. It fetches the challenge image, cuts it into individual glyphs, matches each one against the bundled glyph bank in `assets/glyphs.bin`, and checks the reading against the bundled dictionary in `assets/words.txt`. A rejected answer triggers a fresh challenge and retries without any manual input.

### Follower Boost
Sends the target link to the follower service and applies the amount selected from the limit menu. Works for both profile and video links.

### Hearts
Sends the target link to the hearts service. The largest available amount from the server is applied automatically.

### Comment Hearts
Fetches the full comment list for the target video across all pages, then opens a live searchable picker. Filter by author name or comment text on every keystroke. The selected comment receives the configured heart amount.

### Views
Sends the target link to the views service with the amount chosen from the limit menu. Cooldown responses are counted down and retried automatically.

### Shares
Sends the target link to the shares service with the amount chosen from the limit menu.

### Favorites
Sends the target link to the favorites service with the amount chosen from the limit menu.

### Live Stream
Sends the target link to the live stream service. Intended for a link that is currently live.

### Repost
Sends the target link to the repost service with the amount chosen from the limit menu.

### Cooldown Handling
When the server responds with a wait time, the bot counts it down and retries the same request. A short pause during a submission skips the countdown entirely and signs in from scratch before retrying. During a search, two consecutive cooldowns in a row trigger the same fresh sign-in. The bot always continues from the same target without reloading the service list.

### Amount Selection
Every service that offers an amount returns its own option list from the server. Nothing about the amounts is hardcoded. The largest available option is used automatically where applicable.

### Proxy Support
Proxies are loaded from `proxy.txt`. HTTP and SOCKS5 are both accepted with or without credentials. Running without proxy is fully supported.

---

## File Structure

```text
Toktok-Boost/
├── src/
│   ├── main.rs                      # Entry point, signal handling, startup sequence
│   ├── api/
│   │   ├── endpoints.rs             # Gate and service API calls
│   │   ├── protocol.rs              # Request signing and session headers
│   │   └── mod.rs
│   ├── captcha/
│   │   ├── solver.rs                # Glyph extraction, matching, dictionary check
│   │   └── mod.rs
│   ├── net/
│   │   ├── client.rs                # HTTP client with cookie jar
│   │   ├── egress.rs                # Proxy pool loader and selector
│   │   ├── fingerprint.rs           # Device fingerprint generation
│   │   └── mod.rs
│   ├── ops/
│   │   ├── feature.rs               # Per-service submission logic
│   │   ├── session.rs               # Session connect and re-auth
│   │   ├── target.rs                # Target link loader and validator
│   │   └── mod.rs
│   ├── parse/
│   │   ├── dashboard.rs             # Service list and limit parser
│   │   ├── dom.rs                   # HTML DOM helpers
│   │   ├── forms.rs                 # Form field extractor
│   │   └── mod.rs
│   ├── support/
│   │   ├── banner.rs                # ASCII banner
│   │   ├── cfg.rs                   # Config loader
│   │   ├── input.rs                 # data.txt reader
│   │   ├── log.rs                   # Colored log output
│   │   └── mod.rs
│   └── ui/
│       ├── menu.rs                  # Interactive service menu
│       ├── picker.rs                # Live searchable comment picker
│       └── mod.rs
├── assets/
│   ├── glyphs.bin                   # Compiled glyph bank for captcha solver
│   ├── words.txt                    # Dictionary for captcha answer validation
│   └── icon.ico                     # Windows binary icon
├── image/
│   └── image.png                    # Preview image shown in README
├── scripts/
│   └── package.sh                   # Package script for release distribution
├── Cargo.toml                       # Project manifest and dependencies
├── Cargo.lock
├── build.rs                         # Build script (embeds icon into Windows binary)
├── Makefile                         # make targets: build, run, release, start, clean, size
├── run.sh                           # Run helper: direct, nohup, screen, tmux, logs, stop
├── bot.rb                           # Interactive downloader script (Ruby)
├── HOWTO.txt                        # Quick start guide
├── config.json                      # Sleep duration between cycles
├── data.txt                         # Single TikTok target link
├── proxy.txt                        # Proxy (optional)
├── LICENSE                          # License file
└── utils/
    └── banner.rb                    # Banner for downloader script
```

---

## Disclaimer

This tool is built for educational and technical exploration purposes. Use it wisely and at your own responsibility.

---

<div align="center">
<img width="100%" alt="footer" src="https://capsule-render.vercel.app/api?type=waving&height=120&section=footer"/>
</div>