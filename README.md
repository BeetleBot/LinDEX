# LinDex

A simple Linux client for Samsung DeX.

Samsung never released a DeX app for Linux. LinDex makes it easy to launch and use DeX on a Linux desktop over USB or local network Wi-Fi (note: this is a limited local network mode via ADB, not native Wi-Fi Direct), with proper keyboard and mouse handling.

---

## Quick Start & Installation

### Linux (Recommended)
Download the latest pre-built package from our **[Releases](https://github.com/BeetleBot/LinDEX/releases)**:

```bash
tar -xzf LinDex-Linux-x64-*.tar.gz
sudo ./install.sh
lindex
```
*(The installer places LinDex in `/usr/share/lindex`, creates the `/usr/bin/lindex` command, adds app launcher icons, and ensures `adb` is installed on your Linux distribution).*

---

## Features

- Launches Samsung DeX on a dedicated virtual display
- Works over wired USB or local network Wi-Fi
- Direct keyboard and mouse forwarding (right-click, scroll, shortcuts)
- Configurable resolution and DPI scaling

---

## Device Support & Co-maintainers

I built this primarily for my personal use and have only tested it on my Galaxy S25+. Because of that, I can't guarantee how it will work on other Samsung devices or One UI versions.

If you run into any issues on your device, please open an issue on GitHub and I'll try my best to help solve it. Also, if you use another Samsung device with DeX capabilities and would like to chip in and help maintain this repo with me, please open an issue to let me know!

---

## Building from Source

### Prerequisites

You need Rust and `adb` installed.

**Fedora:**
```bash
sudo dnf install android-tools
```

**Ubuntu / Debian:**
```bash
sudo apt install adb
```

**Arch Linux:**
```bash
sudo pacman -S android-tools
```

If you don't have Rust:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

*(If `scrcpy` is not installed on your system, LinDex will download the runtime automatically on first launch.)*

### Build & Run

```bash
git clone https://github.com/BeetleBot/LinDEX.git
cd LinDEX
cargo run --release
```

To build an optimized release binary:
```bash
cargo build --release
```
The binary will be generated at `target/release/lindex`.

To package a standalone Linux distribution tarball:
```bash
./Release/linux/build-tarball.sh
```
Artifacts are generated in `Release/artifacts/`.

---

## Quick Start

1. Enable **USB debugging** on your phone (Settings > Developer options > USB debugging).
2. Connect your phone via USB cable and allow debugging when prompted on screen.
3. Open LinDex and click **Launch Samsung DeX**.
4. *(Optional)* To go wireless, click **Switch to Wi-Fi**, wait for it to connect, then unplug the cable.

---

## Keyboard Notes

- **Host Key (`Alt`):** While inside DeX, press **Left Alt** or **Right Alt** to release mouse focus back to your Linux desktop.
- **Windows / Super Key:** On KDE Plasma and GNOME, the desktop environment captures the Windows key for its own menu. If you want it to pass through to DeX:
  - On KDE: Right-click the window titlebar > More Actions > Configure Special Window Settings > Add Property > Ignore global shortcuts -> Force: Yes.

---

## License

[MIT](LICENSE)
