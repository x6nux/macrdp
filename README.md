# macrdp

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![macOS](https://img.shields.io/badge/macOS-14%2B-black.svg)](https://www.apple.com/macos/)
[![Apple Silicon](https://img.shields.io/badge/Apple%20Silicon-Supported-green.svg)](#)

**[English](README_EN.md)** | 中文

**macOS 远程桌面服务端**

原生 macOS RDP 服务端。从 Windows、Linux、iOS 或 Android 远程连接你的 Mac — 支持任何标准 RDP 客户端，如 Windows 远程桌面 (mstsc)、Microsoft Remote Desktop、FreeRDP。

> **为什么选 macrdp？** macOS 没有内置 RDP 服务端，自带的 VNC 又慢又模糊。macrdp 让你的 Mac 拥有一流的远程桌面体验 — 快速、清晰、开箱即用兼容所有 RDP 客户端。

---

## 功能特性

- **标准 RDP 协议** — 兼容任何 RDP 客户端，客户端无需安装特殊软件
- **硬件加速编码** — 通过 Apple VideoToolbox GPU 加速 H.264，Apple Silicon 上低延迟
- **高保真色彩** — AVC444 模式像素级色彩还原（RDP 10）
- **完整键鼠支持** — 104 键映射、数字键盘、修饰键、滚轮，完整输入注入
- **HiDPI / Retina 支持** — 2x/3x 倍率采集，远程 4K 高清显示
- **灵活配置** — 分辨率、帧率、码率、编码器、质量预设，简单 TOML 配置
- **安全连接** — NLA/CredSSP 认证 + 自动生成 TLS 证书
- **锁屏采集** — 锁屏时自动切换 CoreGraphics 回退

---

## 环境要求

- **macOS 14+**（Sonoma 或更高版本）
- **Rust 1.75+**
- 屏幕录制权限（系统设置 > 隐私与安全性）
- 辅助功能权限（用于键盘鼠标注入）

---

## 快速开始

```bash
# 编译
cargo build --release

# 运行
cargo run --release --bin macrdp-server

# 从任意 RDP 客户端连接 → Mac-IP:3389
```

---

## 配置

复制 `config.example.toml` 为 `config.toml` 并按需修改：

```toml
# 网络
port = 13389

# 认证
username = "admin"
password = "123456"

# 显示
width = 0          # 0 = 自动检测
height = 0
frame_rate = 60
hidpi_scale = 2    # Retina 上 2 倍缩放获得 4K

# 编码
quality = "high_quality"    # low_latency / balanced / high_quality
encoder = "hardware"        # hardware (GPU) / software (CPU)
chroma_mode = "avc420"      # avc420 (兼容) / avc444 (最佳画质)
bitrate_mbps = 50           # 目标码率 (Mbps)

# 日志
log_level = "info"          # trace / debug / info / warn / error
```

配置搜索顺序:
1. `./config.toml`
2. `~/.config/macrdp/config.toml`
3. `~/Library/Application Support/macrdp/config.toml`

---

## 项目结构

```
crates/
├── macrdp-server/       主服务端程序
├── macrdp-capture/      屏幕采集
├── macrdp-input/        键鼠注入
├── macrdp-encode/       视频编码
├── ironrdp-server-gfx/  RDP 协议层 (IronRDP fork)
└── ironrdp-acceptor-patched/
                         RDP 连接接受器
```

---

## 致谢

本项目的诞生离不开以下优秀的开源项目，在此致以诚挚的敬意：

- **[IronRDP](https://github.com/Devolutions/IronRDP)** — 纯 Rust RDP 协议实现。macrdp 的协议栈基于 ironrdp-server 的 fork，添加了 GFX/AVC444 扩展。
- **[FreeRDP](https://github.com/FreeRDP/FreeRDP)** — 开源 RDP 参考实现。其 AVC444 双流编码方案和 YUV444 B 区域拆分算法是 macrdp 实现的重要参考。
- **[RustDesk](https://github.com/rustdesk/rustdesk)** — 使用 Rust 编写的开源远程桌面软件。其跨平台屏幕采集和输入注入的架构思路给予了很大启发。

---

## 许可证

本项目采用 **GNU 通用公共许可证 v3.0** 授权 — 详见 [LICENSE](LICENSE)。任何基于本项目的衍生作品必须同样以 GPLv3 开源。

---

<details>
<summary><b>关键词</b></summary>

macOS RDP 服务端, Mac 远程桌面, Mac 远程桌面服务端, 远程桌面协议 macOS, 从 Windows 连接 Mac, 从 Linux 连接 Mac, 从安卓连接 Mac, 远程控制 Mac, Mac 远程访问, Mac 屏幕共享, Apple Silicon 远程桌面, VNC 替代方案, macOS 远程桌面方案, macOS RDP server, Mac remote desktop server, RDP server for Mac, connect to Mac from Windows, remote control Mac, VNC alternative Mac

</details>
