use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub last_device_ip: String,
    pub last_device_port: u16,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub bit_rate_mbps: u32,
    pub max_fps: u32,
    pub use_h265: bool,
    pub forward_audio: bool,
    pub stay_awake: bool,
    pub turn_screen_off: bool,
    pub fullscreen: bool,
    pub capture_system_keys: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            last_device_ip: String::new(),
            last_device_port: 5555,
            width: 1920,
            height: 1080,
            dpi: 200,
            bit_rate_mbps: 20,
            max_fps: 60,
            use_h265: true,
            forward_audio: true,
            stay_awake: true,
            turn_screen_off: true,
            fullscreen: false,
            capture_system_keys: true,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> std::path::PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            let app_dir = config_dir.join("linux-dex");
            let _ = std::fs::create_dir_all(&app_dir);
            app_dir.join("config.json")
        } else {
            std::path::PathBuf::from("config.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(data) = std::fs::read_to_string(&path) {
            if let Ok(config) = serde_json::from_str(&data) {
                return config;
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }
}

pub fn find_bundled_bin_dir() -> std::path::PathBuf {
    // 1. Check relative to current running executable (e.g. portable bundle or AppImage)
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let engine_dir = parent.join("engine");
            if engine_dir.join("scrcpy").exists() {
                return engine_dir;
            }
            let bin_dir = parent.join("bin");
            if bin_dir.join("scrcpy").exists() {
                return bin_dir;
            }
        }
    }

    // 2. Check user local data dir: ~/.local/share/lindex/engine
    if let Some(data_dir) = dirs::data_dir() {
        let engine_dir = data_dir.join("lindex").join("engine");
        if engine_dir.join("scrcpy").exists() {
            return engine_dir;
        }
    }

    // 3. Fallback well-known system paths
    let well_known = [
        "/usr/share/lindex/engine",
        "/usr/local/share/lindex/engine",
        "/opt/lindex/engine",
    ];
    for p in well_known {
        let path = std::path::PathBuf::from(p);
        if path.join("scrcpy").exists() {
            return path;
        }
    }

    // Default to user data dir
    if let Some(data_dir) = dirs::data_dir() {
        data_dir.join("lindex").join("engine")
    } else {
        std::path::PathBuf::from("engine")
    }
}

pub fn find_binary(binary_name: &str) -> String {
    // 1. Check system PATH first
    if let Ok(output) = std::process::Command::new("which").arg(binary_name).output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() && std::path::Path::new(&path).exists() {
                return path;
            }
        }
    }

    // 2. Check bundled engine directory
    let engine_dir = find_bundled_bin_dir();
    let candidate = engine_dir.join(binary_name);
    if candidate.exists() {
        return candidate.to_string_lossy().to_string();
    }

    binary_name.to_string()
}

pub fn ensure_scrcpy_engine() -> Result<std::path::PathBuf, String> {
    // 1. If scrcpy is installed on the host system PATH, we're ready
    let system_bin = find_binary("scrcpy");
    if std::path::Path::new(&system_bin).exists() && system_bin != "scrcpy" {
        return Ok(find_bundled_bin_dir());
    }

    // 2. If already provisioned in engine directory
    let engine_dir = find_bundled_bin_dir();
    if engine_dir.join("scrcpy").exists() && engine_dir.join("scrcpy-server").exists() {
        return Ok(engine_dir);
    }

    // 3. Auto-download official scrcpy 5.0.1 for Linux into ~/.local/share/lindex/engine
    let target_dir = dirs::data_dir()
        .map(|d| d.join("lindex").join("engine"))
        .unwrap_or_else(|| std::path::PathBuf::from("engine"));

    let _ = std::fs::create_dir_all(&target_dir);

    let url = "https://github.com/Genymobile/scrcpy/releases/download/v5.0.1/scrcpy-linux-x86_64-v5.0.1.tar.gz";
    let cmd = format!(
        "curl -L -s '{}' | tar -xz --strip-components=1 -C '{}' && chmod +x '{}'",
        url,
        target_dir.display(),
        target_dir.join("scrcpy").display()
    );

    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(&cmd)
        .status()
        .map_err(|e| format!("Failed to download scrcpy engine: {}", e))?;

    if status.success() && target_dir.join("scrcpy").exists() {
        // Clean unneeded auxiliary files from downloaded archive
        let _ = std::fs::remove_file(target_dir.join("adb"));
        let _ = std::fs::remove_file(target_dir.join("scrcpy.1"));
        let _ = std::fs::remove_file(target_dir.join("scrcpy.png"));
        let _ = std::fs::remove_file(target_dir.join("disconnected.png"));
        Ok(target_dir)
    } else {
        Err("Failed to provision scrcpy streaming engine".to_string())
    }
}
