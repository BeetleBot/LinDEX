#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DIST_DIR="${ROOT_DIR}/dist/LinDex-linux-x86_64"
ENGINE_DIR="${DIST_DIR}/engine"

echo "==> 1. Building LinDex in release mode..."
cargo build --release --manifest-path="${ROOT_DIR}/Cargo.toml"

echo "==> 2. Preparing distribution directory at ${DIST_DIR}..."
rm -rf "${DIST_DIR}"
mkdir -p "${DIST_DIR}" "${ENGINE_DIR}"

# Copy application binary
cp "${ROOT_DIR}/target/release/linux-dex" "${DIST_DIR}/LinDex"
chmod +x "${DIST_DIR}/LinDex"

# Copy assets
mkdir -p "${DIST_DIR}/assets"
cp -r "${ROOT_DIR}/assets/"* "${DIST_DIR}/assets/"

echo "==> 3. Bundling scrcpy engine..."
CACHE_TAR="/tmp/scrcpy-linux-x86_64-v5.0.1.tar.gz"
if [ ! -f "${CACHE_TAR}" ]; then
    echo "Downloading official scrcpy 5.0.1 engine for Linux..."
    curl -L -s "https://github.com/Genymobile/scrcpy/releases/download/v5.0.1/scrcpy-linux-x86_64-v5.0.1.tar.gz" -o "${CACHE_TAR}"
fi

tar -xz --strip-components=1 -C "${ENGINE_DIR}" -f "${CACHE_TAR}"
chmod +x "${ENGINE_DIR}/scrcpy"

# Clean non-engine artifacts from scrcpy tarball
rm -f "${ENGINE_DIR}/adb" "${ENGINE_DIR}/scrcpy.1" "${ENGINE_DIR}/scrcpy.png" "${ENGINE_DIR}/disconnected.png"

# Create portable launcher desktop file
cat > "${DIST_DIR}/LinDex.desktop" << 'EOF'
[Desktop Entry]
Name=LinDex
GenericName=Wireless Samsung DeX for Linux
Comment=Wireless Samsung DeX Desktop Client for Linux
Exec=./LinDex
Icon=assets/icon.png
Terminal=false
Type=Application
Categories=Utility;HardwareSettings;System;
Keywords=LinDex;Samsung;DeX;Wireless;Android;Screen;
EOF

echo "==> LinDex successfully bundled at: ${DIST_DIR}"
echo "    Everything is self-contained. You can tarball or ship this folder directly!"
