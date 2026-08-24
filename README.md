<div align="center">

<img src="icon.png" width="128" height="128" alt="MagicToys Logo" />

# 🧩 MagicToys for Linux

### *The Native, Ultra-Fast PowerToys Productivity Suite for Linux*

[![Release](https://img.shields.io/badge/Release-v0.0.4-brightgreen?style=for-the-badge&logo=github)](https://github.com/HeyAtulVerma/MagicToys/releases)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue?style=for-the-badge)](LICENSE)
[![Built With](https://img.shields.io/badge/Built_With-Rust_%26_Slint-orange?style=for-the-badge&logo=rust)](https://slint.dev/)
[![Platform](https://img.shields.io/badge/Platform-X11_%7C_Wayland_%7C_Hyprland-blueviolet?style=for-the-badge&logo=linux)](https://github.com/HeyAtulVerma/MagicToys)
[![Architecture](https://img.shields.io/badge/Arch-x86__64_%7C_amd64-informational?style=for-the-badge)](#)

<p align="center">
  <b>MagicToys</b> brings the most loved productivity tools from Windows PowerToys natively to Linux desktops.<br>
  Engineered in <b>100% Pure Rust and Slint</b> for instant startup, zero background CPU usage, and rock-solid stability.
</p>

---

</div>

## 🌟 Core Features

| Feature | Shortcut | Windows Equivalent | Description |
| :--- | :---: | :---: | :--- |
| **📋 Clipboard History** | <kbd>Alt</kbd> + <kbd>V</kbd> | `Win + V` | Instant recall of copied text, rich formatted text, and PNG image thumbnail previews with smart de-duplication, search, pinning, and SQLite persistence. |
| **😀 Emoji & Symbol Picker** | <kbd>Alt</kbd> + <kbd>.</kbd> | `Win + .` | Categorized Unicode 15+ emoji and symbol grid with live search, keyword tagging, and LRU frequency tracking. |
| **🔍 Windows 11 Text Extractor (OCR)** | <kbd>Alt</kbd> + <kbd>Shift</kbd> + <kbd>T</kbd> | `Win + Shift + T` | Instantaneous screen freeze with dark veil dimming, precision crosshair cursor, Fluent top pill banner, live drag box, and pure-Rust high-contrast OCR directly to clipboard. |

---

## ⚡ Instant Installation

Pre-compiled packages for all major Linux distributions are available under [Releases](releases/):

### 🐧 Ubuntu / Debian / Pop!_OS / Linux Mint / Zorin OS (`.deb`)
```bash
sudo apt install ./releases/magictoys_0.0.4_amd64.deb
```

### 🏹 Arch Linux / Manjaro / EndeavourOS (`.pkg.tar.zst`)
```bash
sudo pacman -U ./releases/magictoys-0.0.4-1-x86_64.pkg.tar.zst
```

### 🎩 Fedora / RHEL / CentOS / openSUSE (`.rpm`)
```bash
sudo dnf install ./releases/magictoys-0.0.4-1.x86_64.rpm
```

### 📦 Universal Standalone AppImage *(Works on any distro)*
```bash
chmod +x releases/MagicToys-0.0.4-x86_64.AppImage
./releases/MagicToys-0.0.4-x86_64.AppImage
```

### 🗜️ Generic Portable Tarball
```bash
tar -xzf releases/magictoys-0.0.4-x86_64.tar.gz
cd magictoys-0.0.4
sudo ./install.sh
```

---

## 🎯 What Makes MagicToys Different?

- 🚀 **Sub-Millisecond Response**: Written in native compiled Rust with GPU-accelerated Slint rendering for 60fps animations and instant popup drawer response.
- 🛡️ **Zero-Crash Resilience**: If optional system tools (`tesseract`, `wtype`, `xclip`, or system tray services) are missing, features degrade gracefully with informative guidance rather than crashing.
- 🌐 **Universal Display Server & DE Support**: 100% functional on X11, native Wayland, Hyprland, Sway, GNOME 45/46/50, KDE Plasma 5/6, XFCE, Cinnamon, and MATE.
- ⌨️ **Multi-Tier Keystroke Simulation**: Intelligent paste simulation automatically selects `wtype`, `ydotool`, `dotool`, direct Linux `/dev/uinput` virtual keyboard, `xdotool`, or XTest based on your active session.
- 🎨 **Modern Fluent UI & Theming**: Auto-adapts to your desktop's Dark/Light mode via Freedesktop DBus protocols, with 6 curated accent colors.
- 🔒 **100% Local & Private**: No telemetry, no network calls, no cloud tracking. All history and preferences remain encrypted in your local `~/.config/magictoys/` directory.

---

## ⌨️ Global Shortcuts & Window Managers

### Desktop Environments (GNOME, KDE Plasma, XFCE, Cinnamon, MATE)
Shortcuts are registered automatically upon installation. You can re-bind or inspect them anytime inside MagicToys Preferences.

| Action | Shortcut | CLI Command |
| :--- | :---: | :--- |
| **Toggle Clipboard History** | <kbd>Alt</kbd> + <kbd>V</kbd> | `magictoys --toggle` |
| **Open Emoji & Symbol Picker** | <kbd>Alt</kbd> + <kbd>.</kbd> | `magictoys --emoji` |
| **Screen Text Extractor (OCR)** | <kbd>Alt</kbd> + <kbd>Shift</kbd> + <kbd>T</kbd> | `magictoys --ocr` |
| **Open Preferences** | — | `magictoys` |
| **Print Version** | — | `magictoys --version` |

---

### Tiling Window Managers (Hyprland / Sway / i3)

Add these bindings to your window manager configuration:

#### **Hyprland** (`~/.config/hypr/hyprland.conf`)
```ini
bind = ALT, V, exec, magictoys --toggle
bind = ALT, PERIOD, exec, magictoys --emoji
bind = ALT SHIFT, T, exec, magictoys --ocr
exec-once = magictoys --background
```

#### **Sway** (`~/.config/sway/config`)
```ini
bindsym Mod1+v exec magictoys --toggle
bindsym Mod1+period exec magictoys --emoji
bindsym Mod1+Shift+t exec magictoys --ocr
exec magictoys --background
```

#### **i3wm** (`~/.config/i3/config`)
```ini
bindsym Mod1+v exec magictoys --toggle
bindsym Mod1+period exec magictoys --emoji
bindsym Mod1+Shift+t exec magictoys --ocr
exec --no-startup-id magictoys --background
```

---

## 🛠️ Building from Source

### Prerequisites
- **Rust toolchain** (1.75+): `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Build essentials**: `make`, `gcc`, `pkg-config`

### Compilation & System Installation
```bash
git clone https://github.com/HeyAtulVerma/MagicToys.git
cd MagicToys

# Build release binary
make build

# Install binary, desktop files, icons, and autostart launchers
sudo make install

# Configure uinput permissions for Wayland keystroke simulation (optional)
sudo make install-rules
```

### Packaging for All Linux Distros
```bash
./build-all.sh
```
*Compiles release binaries with cross-distro GLIBC compatibility and generates `.deb`, `.pkg.tar.zst`, `.rpm`, `.AppImage`, `.tar.gz`, and `SHA256SUMS` in `releases/`.*

---

## 🗑️ Uninstallation

To remove MagicToys from your system:

```bash
# Ubuntu / Debian
sudo apt remove magictoys

# Arch Linux
sudo pacman -R magictoys

# Fedora
sudo dnf remove magictoys

# Source installation
sudo make uninstall
```

---

## 📄 License & Attribution

MagicToys is free and open-source software licensed under the **[GNU General Public License v3.0](LICENSE)** (GPL-3.0-or-later).

*Created with ❤️ for the Linux desktop community.*
