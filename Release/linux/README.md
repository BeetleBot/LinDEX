# Linux Release Build — LinDex

Builds a `.tar.gz` tarball for Linux distribution.

## Prerequisites

- Rust toolchain (`cargo`, `rustc`)

## Quick Start

```bash
chmod +x Release/linux/build-tarball.sh
./Release/linux/build-tarball.sh
```

This will:
1. Build the Rust binary with `cargo build --release`
2. Bundle the `scrcpy` streaming engine under `usr/share/lindex/engine/`
3. Package a `.tar.gz` with binary, engine, icons, desktop file, and install scripts
4. Output → `Release/artifacts/LinDex-Linux-x64-<version>.tar.gz`

## Output

```
Release/artifacts/
  LinDex-Linux-x64-<version>.tar.gz
```

Inside the tarball:
```
usr/share/lindex/lindex                     → Main binary
usr/share/lindex/engine/scrcpy              → Bundled streaming engine
usr/share/lindex/engine/scrcpy-server       → Bundled Android APK server
usr/share/applications/lindex.desktop       → Desktop entry
usr/share/icons/hicolor/.../apps/lindex.png → App icons
install.sh                                  → Installs to /usr/share & installs adb if needed
uninstall.sh                                → Removes from /usr/share
readme.txt                                  → User guide
```

## User Installation

```bash
tar -xzf LinDex-Linux-x64-<version>.tar.gz
sudo ./install.sh
lindex
```

## Uninstallation

```bash
sudo ./uninstall.sh
```
