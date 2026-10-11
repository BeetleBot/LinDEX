#!/bin/sh

if ! [ $# = 0 ]; then
	echo "LinDex .tar.gz uninstaller"
	echo "Usage: sudo ./uninstall.sh"
	exit
fi

echo "Uninstalling LinDex..."

if ! [ $(id -u) = 0 ]; then
	echo "Please re-run using sudo: sudo ${0}" 
	exit 1
fi

echo "Removing program files..."
rm -rf /usr/share/lindex

if [ -L "/usr/bin/lindex" ]; then
	echo "Removing symlink..."
	rm /usr/bin/lindex
fi

if [ -f "/usr/local/bin/lindex" ]; then
	echo "Removing old binary from /usr/local/bin/lindex..."
	rm -f /usr/local/bin/lindex
fi

if command -v xdg-icon-resource >/dev/null 2>&1; then
	echo "Uninstalling application icon..."
	xdg-icon-resource uninstall --size 256 lindex 2>/dev/null || true
fi

if command -v xdg-desktop-menu >/dev/null 2>&1; then
	echo "Removing from menu..."
	xdg-desktop-menu uninstall --novendor usr/share/applications/lindex.desktop 2>/dev/null || true
fi

if [ -e "/usr/share/applications/lindex.desktop" ]; then
	echo "Removing .desktop files..."
	rm -f /usr/share/applications/lindex.desktop /usr/share/applications/LinDex.desktop
fi

rm -f /usr/share/icons/hicolor/*/apps/lindex.png

if command -v update-desktop-database >/dev/null 2>&1; then
	update-desktop-database /usr/share/applications 2>/dev/null || true
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
	gtk-update-icon-cache /usr/share/icons/hicolor 2>/dev/null || true
fi

echo "LinDex uninstallation complete!"
