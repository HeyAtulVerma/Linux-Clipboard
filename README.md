<div align="center">

<img src="icon.png" width="96" height="96" alt="MagicToys" />

# MagicToys

A lightweight productivity tool suite for Linux bringing PowerToys features to X11 and Wayland.

[![Release](https://img.shields.io/badge/Release-v0.0.5-brightgreen)](https://github.com/HeyAtulVerma/MagicToys/releases)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPL--3.0-blue)](LICENSE)

</div>

## Features

| Feature | Shortcut | Description |
| :--- | :---: | :--- |
| **Clipboard History** | <kbd>Alt</kbd> + <kbd>V</kbd> | Text, images, search, and pinning |
| **Emoji Picker** | <kbd>Alt</kbd> + <kbd>.</kbd> | Unicode emoji and symbols |
| **Text Extractor (OCR)** | <kbd>Alt</kbd> + <kbd>Shift</kbd> + <kbd>T</kbd> | Select any screen area to copy text |
| **Color Picker** | <kbd>Alt</kbd> + <kbd>Shift</kbd> + <kbd>C</kbd> | Pick screen colors (HEX, RGB, HSL, CMYK) |

---

## Installation

### Quick Install (Universal)
```bash
curl -sS https://magictoys.ople.in/install.sh | bash
```

### Distro Packages

Pre-built packages are available in [releases/](releases/):

- **Fedora / RHEL / openSUSE**: `sudo dnf install ./releases/magictoys-0.0.5-1.x86_64.rpm`
- **Ubuntu / Debian / Linux Mint**: `sudo apt install ./releases/magictoys_0.0.5_amd64.deb`
- **Arch Linux / Manjaro**: `sudo pacman -U ./releases/magictoys-0.0.5-1-x86_64.pkg.tar.zst`
- **AppImage**: `chmod +x releases/MagicToys-0.0.5-x86_64.AppImage && ./releases/MagicToys-0.0.5-x86_64.AppImage`

---

## Build from Source

```bash
git clone https://github.com/HeyAtulVerma/MagicToys.git
cd MagicToys
make build
sudo make install
```

---

## Contributing & Support

If you find MagicToys useful, please consider giving the project a ⭐️ **star on GitHub**!

Contributions, bug reports, and feature suggestions are always welcome. Feel free to open an issue or submit a pull request on [GitHub](https://github.com/HeyAtulVerma/MagicToys).

---

## License

GPL-3.0
