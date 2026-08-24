# MagicToys Makefile
# Automates compiling, installing, and configuring permissions across Linux distros

PREFIX ?= /usr/local
BINDIR := $(PREFIX)/bin
DATADIR := $(PREFIX)/share
DESTDIR ?=

APP_NAME := magictoys
CARGO_BIN := magictoys
DESKTOP_FILE := magictoys.desktop

.PHONY: all build install uninstall clean install-rules add-user-group

all: build

build:
	cargo build --release

install-rules:
	@echo "Configuring udev permissions for /dev/uinput..."
	@if [ -w /etc/udev/rules.d ]; then \
		echo 'KERNEL=="uinput", MODE="0666", TAG+="uaccess"' > /etc/udev/rules.d/99-magictoys-uinput.rules; \
	else \
		echo 'KERNEL=="uinput", MODE="0666", TAG+="uaccess"' | sudo tee /etc/udev/rules.d/99-magictoys-uinput.rules; \
		sudo modprobe uinput || true; \
		sudo udevadm control --reload-rules || true; \
		sudo udevadm trigger || true; \
	fi
	@echo "✓ udev rules installed successfully."

add-user-group:
	@echo "Adding active user to input group..."
	@if [ -n "$$SUDO_USER" ]; then \
		sudo usermod -aG input $$SUDO_USER; \
		echo "✓ Added $$SUDO_USER to input group"; \
	else \
		sudo usermod -aG input $$USER 2>/dev/null || true; \
		echo "✓ Added $$USER to input group"; \
	fi
	@echo "⚠️ NOTE: You may need to log out and log back in for group changes to take effect."

install:
	@echo "Installing $(APP_NAME) binary..."
	install -Dm755 target/release/$(CARGO_BIN) $(DESTDIR)$(BINDIR)/$(APP_NAME)
	ln -sf $(APP_NAME) $(DESTDIR)$(BINDIR)/MagicToys
	ln -sf $(APP_NAME) $(DESTDIR)$(BINDIR)/lincb.ople.in
	ln -sf $(APP_NAME) $(DESTDIR)$(BINDIR)/linux-clipboard

	@echo "Installing desktop icons..."
	install -Dm644 icon.png $(DESTDIR)$(DATADIR)/icons/hicolor/256x256/apps/$(APP_NAME).png
	install -Dm644 icon.png $(DESTDIR)$(DATADIR)/pixmaps/$(APP_NAME).png

	@echo "Installing desktop launcher..."
	@mkdir -p $(DESTDIR)$(DATADIR)/applications
	@printf "[Desktop Entry]\n\
Name=MagicToys\n\
Comment=Native clipboard, emoji, and OCR tools for Linux\n\
Exec=$(BINDIR)/$(APP_NAME)\n\
Icon=$(APP_NAME)\n\
Terminal=false\n\
Type=Application\n\
Categories=Utility;\n\
StartupNotify=false\n\
StartupWMClass=magictoys\n" > $(DESTDIR)$(DATADIR)/applications/$(DESKTOP_FILE)
	chmod 644 $(DESTDIR)$(DATADIR)/applications/$(DESKTOP_FILE)

	@# Configure autostart
	@mkdir -p $(DESTDIR)/etc/xdg/autostart
	cp $(DESTDIR)$(DATADIR)/applications/$(DESKTOP_FILE) $(DESTDIR)/etc/xdg/autostart/$(DESKTOP_FILE)

	@# Update desktop and icon databases
	@which gtk-update-icon-cache >/dev/null 2>&1 && gtk-update-icon-cache -f -t $(DESTDIR)$(DATADIR)/icons/hicolor 2>/dev/null || true
	@which update-desktop-database >/dev/null 2>&1 && update-desktop-database $(DESTDIR)$(DATADIR)/applications 2>/dev/null || true

	@echo "✓ Installed successfully! You can run it via your application launcher or by typing '$(APP_NAME)'."

uninstall:
	@echo "Removing $(APP_NAME)..."
	rm -f $(DESTDIR)$(BINDIR)/$(APP_NAME)
	rm -f $(DESTDIR)$(BINDIR)/MagicToys
	rm -f $(DESTDIR)$(BINDIR)/lincb.ople.in
	rm -f $(DESTDIR)$(BINDIR)/linux-clipboard
	rm -f $(DESTDIR)$(DATADIR)/applications/$(DESKTOP_FILE)
	rm -f $(DESTDIR)/etc/xdg/autostart/$(DESKTOP_FILE)
	rm -f $(DESTDIR)$(DATADIR)/icons/hicolor/256x256/apps/$(APP_NAME).png
	rm -f $(DESTDIR)$(DATADIR)/pixmaps/$(APP_NAME).png
	rm -f /etc/udev/rules.d/99-magictoys-uinput.rules
	@echo "✓ Uninstalled successfully."

clean:
	cargo clean
