# macrdp

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![macOS](https://img.shields.io/badge/macOS-14%2B-black.svg)](https://www.apple.com/macos/)
[![Apple Silicon](https://img.shields.io/badge/Apple%20Silicon-Supported-green.svg)](#)

English | **[中文](README.md)**

**macOS Remote Desktop Server**

A native RDP server for macOS. Remote into your Mac from Windows, Linux, iOS, or Android — using any standard RDP client like Windows Remote Desktop (mstsc), Microsoft Remote Desktop, or FreeRDP.

> **Why macrdp?** macOS has no built-in RDP server. VNC is slow and blurry. macrdp gives your Mac a first-class remote desktop experience — fast, sharp, and compatible with every RDP client out of the box.

---

## Features

- **Standard RDP protocol** — works with any RDP client, no special software needed on the client side
- **Hardware-accelerated encoding** — GPU-powered H.264 via Apple VideoToolbox, low latency on Apple Silicon
- **High fidelity color** — AVC444 mode for pixel-perfect color reproduction (RDP 10)
- **Full keyboard & mouse** — complete input injection with 104-key mapping, numpad, modifiers, scroll
- **HiDPI / Retina support** — capture at 2x/3x resolution for sharp 4K remote display
- **Configurable** — resolution, frame rate, bitrate, encoder, quality presets, all via simple TOML config
- **Secure** — NLA/CredSSP authentication with auto-generated TLS certificates
- **Lock screen capture** — automatic CoreGraphics fallback when the screen is locked

---

## Requirements

- **macOS 14+** (Sonoma or later)
- **Rust 1.75+**
- Screen Recording permission (System Settings > Privacy & Security)
- Accessibility permission (for keyboard/mouse injection)

---

## Quick Start

```bash
# Build
cargo build --release

# Run
cargo run --release --bin macrdp-server

# Connect from any RDP client → your-mac-ip:3389
```

---

## Configuration

Copy `config.example.toml` to `config.toml` and edit as needed:

```toml
# Network
port = 13389

# Authentication
username = "admin"
password = "123456"

# Display
width = 0          # 0 = auto-detect
height = 0
frame_rate = 60
hidpi_scale = 2    # 2x for 4K on Retina

# Encoding
quality = "high_quality"    # low_latency / balanced / high_quality
encoder = "hardware"        # hardware (GPU) / software (CPU)
chroma_mode = "avc420"      # avc420 (compatible) / avc444 (best quality)
bitrate_mbps = 50           # target bitrate (Mbps)

# Logging
log_level = "info"          # trace / debug / info / warn / error
```

Config search order:
1. `./config.toml`
2. `~/.config/macrdp/config.toml`
3. `~/Library/Application Support/macrdp/config.toml`

---

## Project Structure

```
crates/
├── macrdp-server/       Main server binary
├── macrdp-capture/      Screen capture
├── macrdp-input/        Keyboard & mouse injection
├── macrdp-encode/       Video encoding
├── ironrdp-server-gfx/  RDP protocol (IronRDP fork)
└── ironrdp-acceptor-patched/
                         RDP connection acceptor
```

---

## Acknowledgments

This project stands on the shoulders of giants. Special thanks to:

- **[IronRDP](https://github.com/Devolutions/IronRDP)** — Pure Rust RDP protocol implementation. macrdp's protocol stack is built on a fork of ironrdp-server with GFX/AVC444 extensions.
- **[FreeRDP](https://github.com/FreeRDP/FreeRDP)** — The reference open-source RDP implementation. Its AVC444 dual-stream encoding approach and YUV444 B-area split algorithm were essential references.
- **[RustDesk](https://github.com/rustdesk/rustdesk)** — Open-source remote desktop software written in Rust. Its architecture for cross-platform screen capture and input injection was a great source of inspiration.

---

## License

This project is licensed under the **GNU General Public License v3.0** — see [LICENSE](LICENSE) for details. Any derivative work must also be distributed under GPLv3.

---

<details>
<summary><b>Keywords</b></summary>

macOS RDP server, Mac remote desktop server, RDP server for Mac, remote desktop protocol macOS, connect to Mac from Windows, connect to Mac from Linux, connect to Mac from Android, Windows Remote Desktop to Mac, mstsc Mac, Mac remote access, Mac screen sharing, remote control Mac, Apple Silicon remote desktop, Rust RDP server, VNC alternative Mac, FreeRDP Mac server, macOS remote desktop, Mac remote desktop solution, RDP for macOS

</details>
