fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Automatically provision scrcpy streaming engine into user data dir if missing
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg(r#"
            ENGINE_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/lindex/engine"
            if [ ! -f "$ENGINE_DIR/scrcpy" ] && ! command -v scrcpy >/dev/null 2>&1; then
                echo "cargo:warning=Provisioning scrcpy streaming engine..."
                mkdir -p "$ENGINE_DIR"
                curl -L -s https://github.com/Genymobile/scrcpy/releases/download/v5.0.1/scrcpy-linux-x86_64-v5.0.1.tar.gz | tar -xz --strip-components=1 -C "$ENGINE_DIR"
                chmod +x "$ENGINE_DIR/scrcpy"
                rm -f "$ENGINE_DIR/adb" "$ENGINE_DIR/scrcpy.1" "$ENGINE_DIR/scrcpy.png" "$ENGINE_DIR/disconnected.png"
            fi
        "#)
        .status();
}
