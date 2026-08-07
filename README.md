# 🧩 Ople MagicToys for Linux

<p align="center">
  <img src="icon.png" width="128" height="128" alt="Ople MagicToys logo" />
</p>

<h3 align="center">Native Windows PowerToys &amp; Windows 11 Clipboard History Suite for Linux</h3>

<p align="center">
  <a href="https://magictoys.ople.in"><img src="https://img.shields.io/badge/Website-magictoys.ople.in-0078D4?style=for-the-badge&logo=firefox" alt="Website" /></a>
  <a href="https://github.com/HeyAtulVerma/MagicToys/releases"><img src="https://img.shields.io/badge/Release-v0.0.3--beta-brightgreen?style=for-the-badge&logo=github" alt="Release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-blue?style=for-the-badge" alt="License" /></a>
  <img src="https://img.shields.io/badge/Built_With-Rust_%26_Slint-orange?style=for-the-badge&logo=rust" alt="Rust & Slint" />
</p>

---

## ⚡ Quick One-Line Installation

Install **MagicToys** automatically on **any Linux distribution** (Ubuntu, Debian, Arch, Fedora, Mint, Pop!_OS, openSUSE) with a single command:

```bash
curl -sS https://magictoys.ople.in/install.sh | bash
```

> **Note:** The installer automatically detects your Linux distribution, downloads the matching pre-built package (`.deb`, `.pkg.tar.zst`, `.rpm`, or `AppImage`), sets up system dependencies (`xclip`, `tesseract-ocr`), configures `uinput` permissions for Wayland, and creates desktop launcher shortcuts automatically.

---

## ✨ PowerToys Feature Matrix

**Ople MagicToys** brings popular Windows 11 and Windows PowerToys features natively to Linux:

| Feature | Shortcut | Windows Equivalent | Description |
| :--- | :--- | :--- | :--- |
| **Clipboard History** | <kbd>Alt</kbd> + <kbd>V</kbd> | `Win + V` | Instant recall of copied text, rich formatted text, and PNG image thumbnail previews with smart de-duplication and SQLite persistence. |
| **Emoji &amp; Symbol Picker** | <kbd>Alt</kbd> + <kbd>.</kbd> | `Win + .` | Categorized Unicode emoji grid picker with live search and LRU usage frequency tracking. |
| **Screen Text Extractor (OCR)** | <kbd>Alt</kbd> + <kbd>Shift</kbd> + <kbd>T</kbd> | PowerToys OCR (`Win+Shift+T`) | Drag a box over any screen area to grab text from images, slides, videos, or non-copyable PDFs via Tesseract OCR directly into your clipboard. |

---

## 🎨 Additional Capabilities

- 🎨 **6 Curated Accent Colors**: Choose between Ople Orange (`#f97316`), Sapphire Blue (`#3b82f6`), Amethyst Purple (`#8b5cf6`), Emerald Green (`#22c55e`), Rose Red (`#f43f5e`), and Cyan (`#06b6d4`).
- 🌙 **Dark, Light &amp; System Theme Modes**: Automatic theme detection via DBus notifications. Opaque background architecture guarantees 100% sharp GPU subpixel font antialiasing (FemtoVG/Slint).
- 📌 **Item Pinning &amp; Organization**: Lock important clips to top of history.
- ⚡ **Native Wayland &amp; X11 Engine**: Uses Linux `/dev/uinput` virtual keyboard device on Wayland (GNOME, KDE), with XTest and EWMH focus restoration on X11.
- 🔒 **100% Local &amp; Private**: Zero telemetry, zero tracking, zero network calls. All clips and settings stay local inside `~/.config/magictoys/db.db`.

---

## 📦 Manual Package Installation Options

Download pre-built packages from the [Releases Page](https://github.com/HeyAtulVerma/MagicToys/releases):

### 1. Ubuntu / Debian / Pop!_OS / Mint (`.deb`)
```bash
sudo apt update && sudo apt install -y xclip tesseract-ocr
sudo dpkg -i magictoys_0.0.3_amd64.deb
```
*(Includes automatic desktop session auto-launch script right after installation).*

### 2. Arch Linux / Manjaro / EndeavourOS (`.pkg.tar.zst`)
```bash
sudo pacman -U magictoys-0.0.3-1-x86_64.pkg.tar.zst
```

### 3. Fedora / RHEL / openSUSE (`.rpm`)
```bash
sudo dnf install ./magictoys-0.0.3-1.x86_64.rpm
```

### 4. Portable AppImage (Universal)
```bash
chmod +x MagicToys-0.0.3-x86_64.AppImage
./MagicToys-0.0.3-x86_64.AppImage
```

---

## ⌨️ Keyboard Shortcuts &amp; Navigation

| Shortcut | Action |
| :--- | :--- |
| **`Alt + V`** | Toggle Clipboard History modal |
| **`Alt + .`** | Toggle Emoji Picker panel |
| **`Alt + Shift + T`** | Run Screen Region Text Extractor (OCR) |
| **`Up / Down`** | Navigate through clipboard list |
| **`Enter`** | Paste selected clip directly into current active window |
| **`Delete`** | Remove selected clip from history |

---

## 🛠️ Building from Source &amp; Releases

### Prerequisites (Ubuntu/Debian):
```bash
sudo apt update && sudo apt install -y \
    build-essential pkg-config libfontconfig1-dev libx11-dev \
    libxtst-dev libxdo-dev libglib2.0-dev libgtk-3-dev \
    tesseract-ocr libtesseract-dev libleptonica-dev xclip
```

### Build Binary:
```bash
git clone https://github.com/HeyAtulVerma/MagicToys.git
cd MagicToys/work
cargo build --release
```

### Build All Linux Release Packages (`work/releases/`):
```bash
./build-all.sh
```
*Generates `.deb`, `.pkg.tar.zst`, `.rpm`, `.AppImage`, `.tar.gz`, and `SHA256SUMS` inside `work/releases/`.*

---

## 🗑️ Uninstallation

To remove **MagicToys** from your system:

- **Ubuntu / Debian**: `sudo apt remove magictoys`
- **Arch Linux**: `sudo pacman -R magictoys`
- **Fedora / RHEL**: `sudo dnf remove magictoys`

---

## 📄 License

This project is licensed under the [MIT License](LICENSE).  
Created by **[Ople](https://ople.in)** — Official Homepage: **[magictoys.ople.in](https://magictoys.ople.in)**.
