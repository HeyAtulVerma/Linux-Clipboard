#!/bin/bash
# =============================================================================
# MagicToys - Multi-distro Release Build Script
# Licensed under GNU General Public License v3.0 (GPL-3.0-or-later)
# Produces: .deb, Arch .pkg.tar.zst, .rpm, .AppImage, .tar.gz, and SHA256SUMS
# Output Location: releases/
# =============================================================================

set -e

CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RED='\033[0;31m'
BOLD='\033[1m'
RESET='\033[0m'

APP_NAME="magictoys"
PKG_NAME="magictoys"
VERSION="0.0.5"
ARCH="amd64"
ARCH_LINUX="x86_64"
DESCRIPTION="MagicToys — PowerToys alternative for Linux"
MAINTAINER="MagicToys Contributors <atulverma@ople.in>"
HOMEPAGE="https://github.com/HeyAtulVerma/MagicToys"
LICENSE="GPL-3.0-or-later"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK_DIR="$SCRIPT_DIR"
RELEASES_DIR="$WORK_DIR/releases"
BUILD_CACHE="$WORK_DIR/.build_cache"
BINARY="$WORK_DIR/target/release/$APP_NAME"
ICON="$WORK_DIR/icon.png"

# Clean and prepare output directories (preserve compiler cache)
rm -rf "$RELEASES_DIR"
mkdir -p "$RELEASES_DIR"
mkdir -p "$BUILD_CACHE"
mkdir -p "$WORK_DIR/target/release"

log()  { echo -e "${CYAN}[BUILD]${RESET} $*"; }
ok()   { echo -e "${GREEN}  ✓${RESET} $*"; }
warn() { echo -e "${YELLOW}  ⚠${RESET} $*"; }
err()  { echo -e "${RED}  ✗${RESET} $*"; exit 1; }

banner() {
    echo ""
    echo -e "${BOLD}${CYAN}╔══════════════════════════════════════════════════╗${RESET}"
    echo -e "${BOLD}${CYAN}║  MagicToys – Multi-Distro Builder v${VERSION}        ║${RESET}"
    echo -e "${BOLD}${CYAN}║  Licensed under GNU GPL v3.0                     ║${RESET}"
    echo -e "${BOLD}${CYAN}╚══════════════════════════════════════════════════╝${RESET}"
    echo ""
}

# ─── Step 0: Compile Release Binary ─────────────────────────────────────────
compile() {
    log "Compiling release binary with cross-distro GLIBC compatibility..."
    cd "$WORK_DIR"
    export PATH="$BUILD_CACHE/zig:$HOME/.cargo/bin:$PATH"

    if command -v cargo-zigbuild &>/dev/null && [ -x "$BUILD_CACHE/zig/zig" ]; then
        cargo zigbuild --target x86_64-unknown-linux-gnu.2.31 --release
        cp -f "$WORK_DIR/target/x86_64-unknown-linux-gnu/release/$APP_NAME" "$BINARY"
    else
        cargo build --release
    fi

    if [ ! -f "$BINARY" ]; then
        err "Binary not found at $BINARY!"
    fi

    ok "Binary compiled: $BINARY ($(du -sh "$BINARY" | cut -f1))"
}

# ─── Step 1: .deb Package (Debian / Ubuntu / Mint / Pop!_OS / Zorin OS) ─────
build_deb() {
    log "Building .deb package..."
    local PKG_DIR="$BUILD_CACHE/deb_${PKG_NAME}_${VERSION}_amd64"
    rm -rf "$PKG_DIR"

    # Directory tree
    install -dm755 "$PKG_DIR/DEBIAN"
    install -dm755 "$PKG_DIR/usr/bin"
    install -dm755 "$PKG_DIR/usr/share/applications"
    install -dm755 "$PKG_DIR/usr/share/icons/hicolor/256x256/apps"
    install -dm755 "$PKG_DIR/usr/share/pixmaps"
    install -dm755 "$PKG_DIR/etc/xdg/autostart"
    install -dm755 "$PKG_DIR/usr/lib/udev/rules.d"

    # Binary & legacy symlinks
    install -m755 "$BINARY" "$PKG_DIR/usr/bin/$APP_NAME"
    ln -sf "$APP_NAME" "$PKG_DIR/usr/bin/MagicToys"
    ln -sf "$APP_NAME" "$PKG_DIR/usr/bin/lincb.ople.in"
    ln -sf "$APP_NAME" "$PKG_DIR/usr/bin/linux-clipboard"

    # Icons
    install -m644 "$ICON" "$PKG_DIR/usr/share/icons/hicolor/256x256/apps/${APP_NAME}.png"
    install -m644 "$ICON" "$PKG_DIR/usr/share/pixmaps/${APP_NAME}.png"

    # Desktop file
    cat > "$PKG_DIR/usr/share/applications/${APP_NAME}.desktop" << EOF
[Desktop Entry]
Name=MagicToys
Comment=Native clipboard, emoji, and OCR tools for Linux
Exec=/usr/bin/${APP_NAME}
Icon=${APP_NAME}
Terminal=false
Type=Application
Categories=Utility;
StartupNotify=true
StartupWMClass=magictoys
X-GNOME-UsesNotifications=true
SingleMainWindow=true
EOF
    cp "$PKG_DIR/usr/share/applications/${APP_NAME}.desktop" \
       "$PKG_DIR/etc/xdg/autostart/${APP_NAME}.desktop"

    # udev rule
    echo 'KERNEL=="uinput", MODE="0666", TAG+="uaccess"' \
        > "$PKG_DIR/usr/lib/udev/rules.d/99-magictoys-uinput.rules"

    # DEBIAN/control
    INSTALLED_SIZE=$(du -sk "$PKG_DIR" | cut -f1)
    cat > "$PKG_DIR/DEBIAN/control" << EOF
Package: ${PKG_NAME}
Version: ${VERSION}
Architecture: amd64
Maintainer: ${MAINTAINER}
Installed-Size: ${INSTALLED_SIZE}
Depends: tesseract-ocr, tesseract-ocr-eng, xdg-desktop-portal
Recommends: wl-clipboard, xclip, wtype
Section: utils
Priority: optional
Homepage: ${HOMEPAGE}
Description: ${DESCRIPTION}
 MagicToys is a fast, native, lightweight clipboard history manager,
 emoji picker, and screen OCR text extractor for Linux desktops.
EOF

    # DEBIAN/postinst
    cat > "$PKG_DIR/DEBIAN/postinst" << 'EOF'
#!/bin/sh
set -e
if [ "$1" = "configure" ]; then
    modprobe uinput 2>/dev/null || true
    udevadm control --reload-rules 2>/dev/null || true
    udevadm trigger 2>/dev/null || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database /usr/share/applications 2>/dev/null || true
fi

# Automatically launch background daemon for active desktop user so shortcuts work immediately
if [ -n "$SUDO_USER" ]; then
    USER_ID=$(id -u "$SUDO_USER" 2>/dev/null || true)
    if [ -n "$USER_ID" ]; then
        su - "$SUDO_USER" -c "DISPLAY=${DISPLAY:-:0} WAYLAND_DISPLAY=${WAYLAND_DISPLAY:-wayland-0} XDG_RUNTIME_DIR=/run/user/$USER_ID /usr/bin/magictoys --background >/dev/null 2>&1 &" 2>/dev/null || true
    fi
fi
EOF
    chmod 755 "$PKG_DIR/DEBIAN/postinst"

    # DEBIAN/prerm
    cat > "$PKG_DIR/DEBIAN/prerm" << 'EOF'
#!/bin/sh
set -e
pkill -x "magictoys" 2>/dev/null || true
EOF
    chmod 755 "$PKG_DIR/DEBIAN/prerm"

    local OUT="$RELEASES_DIR/${PKG_NAME}_${VERSION}_amd64.deb"

    if command -v dpkg-deb >/dev/null 2>&1; then
        fakeroot dpkg-deb --build "$PKG_DIR" "$OUT" >/dev/null 2>&1 || dpkg-deb --build "$PKG_DIR" "$OUT"
    else
        # Standalone POSIX .deb packaging using tar + ar (works on any Linux distro)
        local DEB_TMP="$BUILD_CACHE/deb_stage"
        rm -rf "$DEB_TMP"
        mkdir -p "$DEB_TMP"
        echo "2.0" > "$DEB_TMP/debian-binary"
        (cd "$PKG_DIR/DEBIAN" && tar -czf "$DEB_TMP/control.tar.gz" ./*)
        (cd "$PKG_DIR" && tar --exclude='./DEBIAN' -czf "$DEB_TMP/data.tar.gz" ./usr ./etc 2>/dev/null || tar -czf "$DEB_TMP/data.tar.gz" usr etc)
        (cd "$DEB_TMP" && ar rcs "$OUT" debian-binary control.tar.gz data.tar.gz)
    fi

    ok "DEB package: $OUT ($(du -sh "$OUT" | cut -f1))"
}

# ─── Step 2: Arch Linux PKGBUILD + .pkg.tar.zst ───────────────────────────
build_arch() {
    log "Building Arch Linux package..."
    local ARCH_DIR="$BUILD_CACHE/arch"
    rm -rf "$ARCH_DIR"
    mkdir -p "$ARCH_DIR"

    local STAGE="$ARCH_DIR/pkg"
    mkdir -p "$STAGE"
    install -dm755 "$STAGE/usr/bin"
    install -dm755 "$STAGE/usr/share/applications"
    install -dm755 "$STAGE/usr/share/icons/hicolor/256x256/apps"
    install -dm755 "$STAGE/usr/share/pixmaps"
    install -dm755 "$STAGE/etc/xdg/autostart"
    install -dm755 "$STAGE/usr/lib/udev/rules.d"

    install -m755 "$BINARY" "$STAGE/usr/bin/$APP_NAME"
    ln -sf "$APP_NAME" "$STAGE/usr/bin/MagicToys"
    ln -sf "$APP_NAME" "$STAGE/usr/bin/lincb.ople.in"
    ln -sf "$APP_NAME" "$STAGE/usr/bin/linux-clipboard"
    install -m644 "$ICON"   "$STAGE/usr/share/icons/hicolor/256x256/apps/${APP_NAME}.png"
    install -m644 "$ICON"   "$STAGE/usr/share/pixmaps/${APP_NAME}.png"

    cat > "$STAGE/usr/share/applications/${APP_NAME}.desktop" << EOF
[Desktop Entry]
Name=MagicToys
Comment=Native clipboard, emoji, and OCR tools for Linux
Exec=/usr/bin/${APP_NAME}
Icon=${APP_NAME}
Terminal=false
Type=Application
Categories=Utility;
StartupNotify=true
StartupWMClass=magictoys
X-GNOME-UsesNotifications=true
SingleMainWindow=true
EOF
    cp "$STAGE/usr/share/applications/${APP_NAME}.desktop" \
       "$STAGE/etc/xdg/autostart/${APP_NAME}.desktop"
    echo 'KERNEL=="uinput", MODE="0666", TAG+="uaccess"' \
        > "$STAGE/usr/lib/udev/rules.d/99-magictoys-uinput.rules"

    chmod 755 "$STAGE" "$STAGE/usr" "$STAGE/usr/bin" "$STAGE/usr/share" \
              "$STAGE/usr/share/applications" "$STAGE/usr/share/icons" \
              "$STAGE/usr/share/icons/hicolor" "$STAGE/usr/share/icons/hicolor/256x256" \
              "$STAGE/usr/share/icons/hicolor/256x256/apps" "$STAGE/usr/share/pixmaps" \
              "$STAGE/etc" "$STAGE/etc/xdg" "$STAGE/etc/xdg/autostart" "$STAGE/usr/lib" \
              "$STAGE/usr/lib/udev" "$STAGE/usr/lib/udev/rules.d" 2>/dev/null || true

    INSTALLED_SIZE=$(du -sk "$STAGE" | cut -f1)
    cat > "$STAGE/.PKGINFO" << EOF
pkgname = ${PKG_NAME}
pkgver = ${VERSION}-1
pkgdesc = ${DESCRIPTION}
url = ${HOMEPAGE}
builddate = $(date +%s)
packager = ${MAINTAINER}
size = $((INSTALLED_SIZE * 1024))
arch = x86_64
license = ${LICENSE}
depend = tesseract
depend = tesseract-data-eng
depend = xdg-desktop-portal
optdepend = wl-clipboard: Wayland clipboard access
optdepend = xclip: X11 clipboard access
optdepend = wtype: Wayland keystroke simulation
EOF

    local PKG_OUT_NAME="${PKG_NAME}-${VERSION}-1-x86_64.pkg.tar.zst"
    if command -v zstd &>/dev/null; then
        (cd "$STAGE" && tar -c --zstd -f "$RELEASES_DIR/$PKG_OUT_NAME" .PKGINFO usr etc 2>/dev/null)
    else
        PKG_OUT_NAME="${PKG_NAME}-${VERSION}-1-x86_64.pkg.tar.xz"
        (cd "$STAGE" && tar -cJf "$RELEASES_DIR/$PKG_OUT_NAME" .PKGINFO usr etc 2>/dev/null)
    fi

    ok "Arch package: $RELEASES_DIR/$PKG_OUT_NAME ($(du -sh "$RELEASES_DIR/$PKG_OUT_NAME" | cut -f1))"
}

# ─── Step 3: .rpm Package (Fedora / RHEL / openSUSE) ──────────────────────
build_rpm() {
    log "Building RPM package..."
    local RPM_DIR="$BUILD_CACHE/rpm"
    local STAGE_DIR="$RPM_DIR/STAGE"

    rm -rf "$RPM_DIR"
    mkdir -p "$RPM_DIR"/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS,tmp} "$STAGE_DIR"
    local SPEC="$RPM_DIR/SPECS/${PKG_NAME}.spec"
    install -dm755 "$STAGE_DIR/usr/bin"
    install -dm755 "$STAGE_DIR/usr/share/applications"
    install -dm755 "$STAGE_DIR/usr/share/icons/hicolor/256x256/apps"
    install -dm755 "$STAGE_DIR/usr/share/pixmaps"
    install -dm755 "$STAGE_DIR/etc/xdg/autostart"
    install -dm755 "$STAGE_DIR/usr/lib/udev/rules.d"

    install -m755 "$BINARY" "$STAGE_DIR/usr/bin/$APP_NAME"
    ln -sf "$APP_NAME" "$STAGE_DIR/usr/bin/MagicToys"
    ln -sf "$APP_NAME" "$STAGE_DIR/usr/bin/lincb.ople.in"
    ln -sf "$APP_NAME" "$STAGE_DIR/usr/bin/linux-clipboard"
    install -m644 "$ICON"   "$STAGE_DIR/usr/share/icons/hicolor/256x256/apps/${APP_NAME}.png"
    install -m644 "$ICON"   "$STAGE_DIR/usr/share/pixmaps/${APP_NAME}.png"

    cat > "$STAGE_DIR/usr/share/applications/${APP_NAME}.desktop" << EOF
[Desktop Entry]
Name=MagicToys
Comment=Native clipboard, emoji, and OCR tools for Linux
Exec=/usr/bin/${APP_NAME}
Icon=${APP_NAME}
Terminal=false
Type=Application
Categories=Utility;
StartupNotify=true
StartupWMClass=magictoys
X-GNOME-UsesNotifications=true
SingleMainWindow=true
EOF
    cp "$STAGE_DIR/usr/share/applications/${APP_NAME}.desktop" \
       "$STAGE_DIR/etc/xdg/autostart/${APP_NAME}.desktop"

    echo 'KERNEL=="uinput", MODE="0666", TAG+="uaccess"' \
        > "$STAGE_DIR/usr/lib/udev/rules.d/99-magictoys-uinput.rules"

    cat > "$SPEC" << EOF
Name:           ${PKG_NAME}
Version:        ${VERSION}
Release:        1%{?dist}
Summary:        ${DESCRIPTION}
License:        ${LICENSE}
URL:            ${HOMEPAGE}
BuildArch:      x86_64

Requires:       tesseract
Requires:       tesseract-langpack-eng
Requires:       xdg-desktop-portal
Recommends:     wl-clipboard
Recommends:     xclip
Recommends:     wtype

%description
MagicToys is a fast, native, lightweight clipboard history manager,
emoji picker, and screen OCR text extractor for Linux desktops.

%install
rm -rf %{buildroot}
mkdir -p %{buildroot}
cp -a ${STAGE_DIR}/* %{buildroot}/

%files
/usr/bin/${APP_NAME}
/usr/bin/MagicToys
/usr/bin/lincb.ople.in
/usr/bin/linux-clipboard
/usr/share/applications/${APP_NAME}.desktop
/usr/share/icons/hicolor/256x256/apps/${APP_NAME}.png
/usr/share/pixmaps/${APP_NAME}.png
/etc/xdg/autostart/${APP_NAME}.desktop
/usr/lib/udev/rules.d/99-magictoys-uinput.rules

%post
if [ \$1 -eq 1 ] || [ \$1 -eq 2 ]; then
    /sbin/ldconfig 2>/dev/null || true
    modprobe uinput 2>/dev/null || true
    udevadm control --reload-rules 2>/dev/null || true
    udevadm trigger 2>/dev/null || true
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t /usr/share/icons/hicolor 2>/dev/null || true
    fi
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database /usr/share/applications 2>/dev/null || true
    fi
    # Automatically launch background daemon for active desktop user
    if [ -n "\$SUDO_USER" ]; then
        USER_ID=\$(id -u "\$SUDO_USER" 2>/dev/null || true)
        if [ -n "\$USER_ID" ]; then
            su - "\$SUDO_USER" -c "DISPLAY=\${DISPLAY:-:0} WAYLAND_DISPLAY=\${WAYLAND_DISPLAY:-wayland-0} XDG_RUNTIME_DIR=/run/user/\$USER_ID /usr/bin/magictoys --background >/dev/null 2>&1 &" 2>/dev/null || true
        fi
    fi
fi

%preun
if [ \$1 -eq 0 ]; then
    pkill -x "magictoys" 2>/dev/null || true
fi
EOF

    if command -v rpmbuild &>/dev/null; then
        rpmbuild --define "_topdir $RPM_DIR" \
                 --define "_tmppath $RPM_DIR/tmp" \
                 --define "_builddir $RPM_DIR/BUILD" \
                 --define "_rpmdir $RPM_DIR/RPMS" \
                 --define "_srcrpmdir $RPM_DIR/SRPMS" \
                 --define "_buildrootdir $RPM_DIR/BUILDROOT" \
                 --nodeps \
                 -bb "$SPEC" &>/dev/null || rpmbuild --define "_topdir $RPM_DIR" --define "_tmppath $RPM_DIR/tmp" --nodeps -bb "$SPEC"

        local RPM_FILE=$(find "$RPM_DIR/RPMS" -name "*${VERSION}*.rpm" | head -n 1)
        if [ -z "$RPM_FILE" ]; then
            RPM_FILE=$(find "$RPM_DIR/RPMS" -name "*.rpm" | head -n 1)
        fi
        if [ -n "$RPM_FILE" ]; then
            cp -f "$RPM_FILE" "$RELEASES_DIR/${PKG_NAME}-${VERSION}-1.x86_64.rpm"
            ok "RPM package: $RELEASES_DIR/${PKG_NAME}-${VERSION}-1.x86_64.rpm ($(du -sh "$RELEASES_DIR/${PKG_NAME}-${VERSION}-1.x86_64.rpm" | cut -f1))"
        else
            warn "rpmbuild ran but no RPM file was produced."
        fi
    else
        warn "rpmbuild not found — skipped RPM build."
    fi
}

# ─── Step 4: AppImage Package ──────────────────────────────────────────────
build_appimage() {
    log "Building AppImage package..."
    local APPDIR="$BUILD_CACHE/AppDir"
    rm -rf "$APPDIR"
    mkdir -p "$APPDIR/usr/bin"
    mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"
    mkdir -p "$APPDIR/usr/share/pixmaps"
    mkdir -p "$APPDIR/usr/share/applications"

    # Install files into AppDir
    install -m755 "$BINARY" "$APPDIR/usr/bin/$APP_NAME"
    ln -sf "$APP_NAME" "$APPDIR/usr/bin/MagicToys"
    ln -sf "$APP_NAME" "$APPDIR/usr/bin/lincb.ople.in"
    ln -sf "$APP_NAME" "$APPDIR/usr/bin/linux-clipboard"
    install -m644 "$ICON"   "$APPDIR/usr/share/icons/hicolor/256x256/apps/${APP_NAME}.png"
    install -m644 "$ICON"   "$APPDIR/usr/share/pixmaps/${APP_NAME}.png"
    install -m644 "$ICON"   "$APPDIR/${APP_NAME}.png"

    cat > "$APPDIR/${APP_NAME}.desktop" << EOF
[Desktop Entry]
Name=MagicToys
Comment=Native clipboard, emoji, and OCR tools for Linux
Exec=${APP_NAME}
Icon=${APP_NAME}
Terminal=false
Type=Application
Categories=Utility;
StartupNotify=true
StartupWMClass=magictoys
X-GNOME-UsesNotifications=true
SingleMainWindow=true
EOF
    cp "$APPDIR/${APP_NAME}.desktop" "$APPDIR/usr/share/applications/${APP_NAME}.desktop"

    # AppRun launcher script
    cat > "$APPDIR/AppRun" << 'EOF'
#!/bin/bash
HERE="$(dirname "$(readlink -f "${BASH_SOURCE[0]}")")"
export PATH="$HERE/usr/bin:$PATH"
export LD_LIBRARY_PATH="$HERE/usr/lib:$LD_LIBRARY_PATH"
exec "$HERE/usr/bin/magictoys" "$@"
EOF
    chmod +x "$APPDIR/AppRun"

    # Check for appimagetool or download if missing
    local TOOL=""
    if command -v appimagetool &>/dev/null; then
        TOOL="appimagetool"
    elif [ -f "$BUILD_CACHE/appimagetool" ]; then
        TOOL="$BUILD_CACHE/appimagetool"
    else
        log "Downloading appimagetool..."
        if wget -q -O "$BUILD_CACHE/appimagetool" "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage" 2>/dev/null || \
           curl -sSL -o "$BUILD_CACHE/appimagetool" "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage" 2>/dev/null; then
            chmod +x "$BUILD_CACHE/appimagetool"
            TOOL="$BUILD_CACHE/appimagetool"
        fi
    fi

    local OUT_APPIMAGE="$RELEASES_DIR/MagicToys-${VERSION}-x86_64.AppImage"
    if [ -n "$TOOL" ] && [ -x "$TOOL" ]; then
        ARCH=x86_64 "$TOOL" --appimage-extract-and-run "$APPDIR" "$OUT_APPIMAGE" &>/dev/null || ARCH=x86_64 "$TOOL" "$APPDIR" "$OUT_APPIMAGE" &>/dev/null || true
    fi

    if [ -f "$OUT_APPIMAGE" ]; then
        ok "AppImage package: $OUT_APPIMAGE ($(du -sh "$OUT_APPIMAGE" | cut -f1))"
    else
        # Create self-contained portable directory bundle as AppImage fallback
        local APPDIR_TAR="$RELEASES_DIR/MagicToys-${VERSION}-x86_64-AppDir.tar.gz"
        (cd "$BUILD_CACHE" && tar -czf "$APPDIR_TAR" AppDir)
        ok "Portable AppDir bundle: $APPDIR_TAR ($(du -sh "$APPDIR_TAR" | cut -f1))"
    fi
}

# ─── Step 5: Generic .tar.gz Portable Archive ─────────────────────────────
build_tarball() {
    log "Building generic .tar.gz archive..."
    local TAR_STAGE="$BUILD_CACHE/tarball/magictoys-${VERSION}"
    rm -rf "$BUILD_CACHE/tarball"
    mkdir -p "$TAR_STAGE"

    install -m755 "$BINARY" "$TAR_STAGE/$APP_NAME"
    ln -sf "$APP_NAME" "$TAR_STAGE/MagicToys"
    ln -sf "$APP_NAME" "$TAR_STAGE/lincb.ople.in"
    ln -sf "$APP_NAME" "$TAR_STAGE/linux-clipboard"
    install -m644 "$ICON"   "$TAR_STAGE/icon.png"

    cat > "$TAR_STAGE/install.sh" << 'EOF'
#!/bin/bash
set -e
PREFIX="${PREFIX:-/usr/local}"
DESTDIR="${DESTDIR:-}"

echo "Installing MagicToys to ${DESTDIR}${PREFIX}..."
install -dm755 "${DESTDIR}${PREFIX}/bin"
install -m755 magictoys "${DESTDIR}${PREFIX}/bin/magictoys"
ln -sf magictoys "${DESTDIR}${PREFIX}/bin/MagicToys"
ln -sf magictoys "${DESTDIR}${PREFIX}/bin/lincb.ople.in"
ln -sf magictoys "${DESTDIR}${PREFIX}/bin/linux-clipboard"

if [ -d "${DESTDIR}${PREFIX}/share" ]; then
    install -dm755 "${DESTDIR}${PREFIX}/share/icons/hicolor/256x256/apps"
    install -m644 icon.png "${DESTDIR}${PREFIX}/share/icons/hicolor/256x256/apps/magictoys.png"
    install -dm755 "${DESTDIR}${PREFIX}/share/pixmaps"
    install -m644 icon.png "${DESTDIR}${PREFIX}/share/pixmaps/magictoys.png"
    install -dm755 "${DESTDIR}${PREFIX}/share/applications"
    cat > "${DESTDIR}${PREFIX}/share/applications/magictoys.desktop" << 'DESK'
[Desktop Entry]
Name=MagicToys
Comment=Native clipboard, emoji, and OCR tools for Linux
Exec=magictoys
Icon=magictoys
Terminal=false
Type=Application
Categories=Utility;Productivity;
StartupNotify=true
StartupWMClass=magictoys
X-GNOME-UsesNotifications=true
SingleMainWindow=true
DESK
fi

echo "✓ MagicToys installed successfully!"
EOF
    chmod +x "$TAR_STAGE/install.sh"

    local OUT_TAR="$RELEASES_DIR/magictoys-${VERSION}-x86_64.tar.gz"
    (cd "$BUILD_CACHE/tarball" && tar -czf "$OUT_TAR" "magictoys-${VERSION}")
    ok "Tarball archive: $OUT_TAR ($(du -sh "$OUT_TAR" | cut -f1))"
}

# ─── Step 6: SHA256 Checksums ──────────────────────────────────────────────
build_checksums() {
    log "Generating SHA256SUMS file..."
    cd "$RELEASES_DIR"
    rm -f SHA256SUMS
    sha256sum * > SHA256SUMS 2>/dev/null || true
    ok "Checksum file: $RELEASES_DIR/SHA256SUMS"
}

# ─── Summary ─────────────────────────────────────────────────────────────
summary() {
    echo ""
    echo -e "${BOLD}${GREEN}╔══════════════════════════════════════════════════╗${RESET}"
    echo -e "${BOLD}${GREEN}║  All Packages Built Successfully!                ║${RESET}"
    echo -e "${BOLD}${GREEN}╚══════════════════════════════════════════════════╝${RESET}"
    echo ""
    echo -e "${BOLD}Output directory:${RESET} $RELEASES_DIR"
    echo ""
    ls -lh "$RELEASES_DIR" | grep -v '^total' | awk '{print "  • " $9 " (" $5 ")"}'
    echo ""
}

main() {
    banner
    compile
    echo ""
    build_deb
    build_arch
    build_rpm
    build_appimage
    build_tarball
    build_checksums
    summary
}

main "$@"
