#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ARTIFACTS_DIR="$PROJECT_ROOT/Release/artifacts"

VERSION=$(grep -oP '^version = "\K[^"]+' "$PROJECT_ROOT/Cargo.toml" | head -1)
echo "==> Building LinDex v$VERSION (Linux Tarball)"

if [ -t 0 ]; then
    echo ""
    echo -n "Enter version number (current is $VERSION, press Enter to keep): "
    read -r NEW_VERSION || true
    if [ -n "${NEW_VERSION:-}" ]; then
        VERSION="$NEW_VERSION"
        echo "==> Updating version to $VERSION"
        sed -i "s/^version = \".*\"/version = \"$VERSION\"/" "$PROJECT_ROOT/Cargo.toml"
        echo "==> Version updated in Cargo.toml"
    fi
fi

echo "==> 1. Building Rust release binary..."
cd "$PROJECT_ROOT"
cargo build --release

echo "==> 2. Preparing tarball directory structure..."
TARBALL_DIR="$PROJECT_ROOT/target/tarball"
rm -rf "$TARBALL_DIR"
mkdir -p "$TARBALL_DIR/usr/share/lindex/engine"
mkdir -p "$TARBALL_DIR/usr/share/applications"

# Copy icons for all standard desktop resolutions
for sz in 1024 512 256 128 64 32; do
    mkdir -p "$TARBALL_DIR/usr/share/icons/hicolor/${sz}x${sz}/apps"
    if [ -f "$PROJECT_ROOT/assets/icon_${sz}.png" ]; then
        cp "$PROJECT_ROOT/assets/icon_${sz}.png" "$TARBALL_DIR/usr/share/icons/hicolor/${sz}x${sz}/apps/lindex.png"
    elif [ "$sz" = "1024" ] && [ -f "$PROJECT_ROOT/assets/icon.png" ]; then
        cp "$PROJECT_ROOT/assets/icon.png" "$TARBALL_DIR/usr/share/icons/hicolor/1024x1024/apps/lindex.png"
    fi
done

# Copy binary & desktop entry
cp "$PROJECT_ROOT/target/release/lindex" "$TARBALL_DIR/usr/share/lindex/"
cp "$PROJECT_ROOT/assets/linux/lindex.desktop" "$TARBALL_DIR/usr/share/applications/"

# Copy bundled scrcpy streaming engine
echo "==> 3. Bundling scrcpy streaming engine..."
LOCAL_ENGINE="${XDG_DATA_HOME:-$HOME/.local/share}/lindex/engine"
if [ -f "$LOCAL_ENGINE/scrcpy" ] && [ -f "$LOCAL_ENGINE/scrcpy-server" ]; then
    cp "$LOCAL_ENGINE/scrcpy" "$TARBALL_DIR/usr/share/lindex/engine/"
    cp "$LOCAL_ENGINE/scrcpy-server" "$TARBALL_DIR/usr/share/lindex/engine/"
else
    CACHE_TAR="/tmp/scrcpy-linux-x86_64-v5.0.1.tar.gz"
    if [ ! -f "$CACHE_TAR" ]; then
        echo "Downloading official scrcpy 5.0.1 for Linux..."
        curl -L -s "https://github.com/Genymobile/scrcpy/releases/download/v5.0.1/scrcpy-linux-x86_64-v5.0.1.tar.gz" -o "$CACHE_TAR"
    fi
    tar -xz --strip-components=1 -C "$TARBALL_DIR/usr/share/lindex/engine" -f "$CACHE_TAR"
    rm -f "$TARBALL_DIR/usr/share/lindex/engine/adb" \
          "$TARBALL_DIR/usr/share/lindex/engine/scrcpy.1" \
          "$TARBALL_DIR/usr/share/lindex/engine/scrcpy.png" \
          "$TARBALL_DIR/usr/share/lindex/engine/disconnected.png"
fi
chmod +x "$TARBALL_DIR/usr/share/lindex/engine/scrcpy" 2>/dev/null || true
chmod +x "$TARBALL_DIR/usr/share/lindex/lindex"

# Copy installer scripts and documentation
cp "$PROJECT_ROOT/assets/linux/install.sh" "$TARBALL_DIR/"
cp "$PROJECT_ROOT/assets/linux/uninstall.sh" "$TARBALL_DIR/"
cp "$PROJECT_ROOT/assets/linux/readme.txt" "$TARBALL_DIR/"
if [ -f "$PROJECT_ROOT/LICENSE" ]; then
    cp "$PROJECT_ROOT/LICENSE" "$TARBALL_DIR/"
fi
chmod +x "$TARBALL_DIR/install.sh" "$TARBALL_DIR/uninstall.sh"

echo "==> 4. Generating distribution tarball..."
TARBALL_FILE="LinDex-Linux-x64-$VERSION.tar.gz"
mkdir -p "$ARTIFACTS_DIR"
cd "$TARBALL_DIR"
tar -czf "$ARTIFACTS_DIR/$TARBALL_FILE" .

rm -rf "$TARBALL_DIR"

echo "==> Done! Artifact created:"
echo "    $ARTIFACTS_DIR/$TARBALL_FILE"
