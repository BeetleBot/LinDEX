#!/bin/sh

if ! [ $# = 0 ]; then
	echo "LinDex .tar.gz installer"
	echo "Usage: sudo ./install.sh"
	exit
fi

echo "Installing LinDex to /usr/share/lindex..."

if ! [ $(id -u) = 0 ]; then
	echo "Please re-run using sudo: sudo ${0}" 
	exit 1
fi

# Install adb if missing based on distro package manager
if ! command -v adb >/dev/null 2>&1; then
	echo "Installing adb dependency..."
	if command -v dnf >/dev/null 2>&1; then
		dnf install -y android-tools || true
	elif command -v apt-get >/dev/null 2>&1; then
		apt-get update -qq && apt-get install -y adb || true
	elif command -v pacman >/dev/null 2>&1; then
		pacman -Sy --noconfirm android-tools || true
	elif command -v zypper >/dev/null 2>&1; then
		zypper --non-interactive install android-tools || true
	fi
fi

echo "Copying program files to /usr/share/..."
cp -R usr/share/lindex /usr/share/

if [ -d "usr/share/icons" ]; then
	echo "Installing icons..."
	cp -R usr/share/icons /usr/share/
fi

echo "Installing desktop launcher..."
if [ -d "usr/share/applications" ]; then
	cp -R usr/share/applications /usr/share/
	ln -sf /usr/share/applications/lindex.desktop /usr/share/applications/LinDex.desktop
fi

echo "Creating symlink /usr/bin/lindex..."
ln -sf /usr/share/lindex/lindex /usr/bin/lindex

if [ -f "/usr/local/bin/lindex" ] && ! [ -L "/usr/local/bin/lindex" ]; then
	echo "Removing old binary from /usr/local/bin/lindex to avoid PATH conflict..."
	rm -f /usr/local/bin/lindex
fi

if command -v update-desktop-database >/dev/null 2>&1; then
	update-desktop-database /usr/share/applications 2>/dev/null || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
	gtk-update-icon-cache /usr/share/icons/hicolor 2>/dev/null || true
fi

echo "LinDex installation complete!"
