use crate::config::AppConfig;
use anyhow::{anyhow, Result};
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};

pub struct DexSession {
    current_process: Arc<Mutex<Option<Child>>>,
}

impl DexSession {
    pub fn new() -> Self {
        Self {
            current_process: Arc::new(Mutex::new(None)),
        }
    }

    pub fn is_running(&self) -> bool {
        let mut guard = self.current_process.lock().unwrap();
        if let Some(child) = guard.as_mut() {
            match child.try_wait() {
                Ok(None) => true,
                _ => {
                    *guard = None;
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn stop(&self) -> Result<()> {
        let mut guard = self.current_process.lock().unwrap();
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        Ok(())
    }

    /// Launch Samsung DeX session in Virtual Display mode
    pub fn launch(&self, device_serial: &str, config: &AppConfig) -> Result<()> {
        if self.is_running() {
            return Err(anyhow!("A DeX session is already active"));
        }

        // Ensure scrcpy engine is available (auto-downloads to ~/.local/share/lindex/engine if missing)
        if let Err(e) = crate::config::ensure_scrcpy_engine() {
            tracing::warn!("Could not auto-provision scrcpy engine: {}", e);
        }
        let scrcpy_bin = crate::config::find_binary("scrcpy");

        let mut args = Vec::new();

        // Target device
        args.push("-s".to_string());
        args.push(device_serial.to_string());

        // Virtual display for Samsung DeX: <width>x<height>/<dpi>
        let display_arg = format!("--new-display={}x{}/{}", config.width, config.height, config.dpi);
        args.push(display_arg);

        // Video codec & bitrate
        let codec = if config.use_h265 { "h265" } else { "h264" };
        args.push(format!("--video-codec={}", codec));
        
        // Snapdragon hardware encoder
        if config.use_h265 {
            args.push("--video-encoder=c2.qti.hevc.encoder".to_string());
        } else {
            args.push("--video-encoder=c2.qti.avc.encoder".to_string());
        }

        args.push(format!("--video-bit-rate={}M", config.bit_rate_mbps));
        args.push(format!("--max-fps={}", config.max_fps));

        // Minimize latency
        args.push("--video-buffer=0".to_string());
        args.push("--audio-buffer=20".to_string());

        // Audio forwarding
        if config.forward_audio {
            args.push("--audio-codec=opus".to_string());
            args.push("--audio-bit-rate=128K".to_string());
        } else {
            args.push("--no-audio".to_string());
        }

        // Forward physical input directly via UHID
        if config.capture_system_keys {
            args.push("--keyboard=uhid".to_string());
            args.push("--mouse=uhid".to_string());
            args.push("--mouse-bind=++++:++++".to_string());
        }

        args.push("--window-title=LinDex (Samsung DeX)".to_string());
        // Use Alt to toggle mouse capture back to host
        args.push("--shortcut-mod=lalt,ralt".to_string());

        if config.stay_awake {
            args.push("--stay-awake".to_string());
        }
        if config.turn_screen_off {
            args.push("--turn-screen-off".to_string());
        }
        if config.fullscreen {
            args.push("--fullscreen".to_string());
        }

        // Environment variables for scrcpy server location
        let engine_dir = crate::config::find_bundled_bin_dir();

        let mut cmd = Command::new(&scrcpy_bin);
        cmd.args(&args);

        if engine_dir.exists() {
            let server_path = engine_dir.join("scrcpy-server");
            if server_path.exists() {
                cmd.env("SCRCPY_SERVER_PATH", &server_path);
            }
            if let Ok(path) = std::env::var("PATH") {
                cmd.env("PATH", format!("{}:{}", engine_dir.display(), path));
            }
        }

        let child = cmd.spawn()
            .map_err(|e| anyhow!("Failed to spawn DeX engine ({}): {}", scrcpy_bin, e))?;

        let mut guard = self.current_process.lock().unwrap();
        *guard = Some(child);

        Ok(())
    }
}
