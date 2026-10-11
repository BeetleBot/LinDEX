use anyhow::{anyhow, Result};
use std::process::Command;
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceTransport {
    Usb,
    Wifi(String), // ip address
}

#[derive(Debug, Clone)]
pub struct AdbDevice {
    pub serial: String,
    #[allow(dead_code)]
    pub state: String,
    pub model: String,
    pub transport: DeviceTransport,
}

pub struct AdbManager {
    adb_bin: String,
}

impl AdbManager {
    pub fn new() -> Self {
        let adb_bin = crate::config::find_binary("adb");
        Self { adb_bin }
    }

    /// List all devices detected by ADB
    pub fn list_devices(&self) -> Result<Vec<AdbDevice>> {
        let output = Command::new(&self.adb_bin)
            .args(["devices", "-l"])
            .output()?;

        if !output.status.success() {
            return Err(anyhow!("Failed to run adb devices"));
        }

        let text = String::from_utf8_lossy(&output.stdout);
        let mut devices = Vec::new();
        let re_model = Regex::new(r"model:([^\s]+)").unwrap();

        for line in text.lines().skip(1) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let serial = parts[0].to_string();
                let state = parts[1].to_string();

                let model = if let Some(caps) = re_model.captures(line) {
                    caps.get(1).map(|m| m.as_str().replace('_', " ")).unwrap_or_else(|| "Android Device".to_string())
                } else {
                    "Android Device".to_string()
                };

                let transport = if serial.contains(':') {
                    let ip = serial.split(':').next().unwrap_or("").to_string();
                    DeviceTransport::Wifi(ip)
                } else {
                    DeviceTransport::Usb
                };

                devices.push(AdbDevice {
                    serial,
                    state,
                    model,
                    transport,
                });
            }
        }

        Ok(devices)
    }

    /// Connect to a device via IP and port
    pub fn connect_wifi(&self, ip: &str, port: u16) -> Result<String> {
        let target = format!("{}:{}", ip, port);
        let output = Command::new(&self.adb_bin)
            .args(["connect", &target])
            .output()?;

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if text.contains("connected to") {
            Ok(text)
        } else {
            Err(anyhow!(text))
        }
    }

    /// Switch a USB connected phone to TCP/IP listening mode on port 5555
    pub fn enable_tcpip(&self, serial: Option<&str>, port: u16) -> Result<String> {
        let mut cmd = Command::new(&self.adb_bin);
        if let Some(s) = serial {
            cmd.args(["-s", s]);
        }
        cmd.args(["tcpip", &port.to_string()]);

        let output = cmd.output()?;
        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(text)
    }

    /// Enable freeform multi-window desktop mode so each app opens in its own window
    pub fn enable_multi_window_desktop(&self, serial: &str) -> Result<()> {
        let _ = Command::new(&self.adb_bin)
            .args(["-s", serial, "shell", "settings", "put", "global", "enable_freeform_support", "1"])
            .output();
        let _ = Command::new(&self.adb_bin)
            .args(["-s", serial, "shell", "settings", "put", "global", "force_resizable_activities", "1"])
            .output();
        let _ = Command::new(&self.adb_bin)
            .args(["-s", serial, "shell", "settings", "put", "global", "force_desktop_mode_on_external_displays", "1"])
            .output();
        let _ = Command::new(&self.adb_bin)
            .args(["-s", serial, "shell", "settings", "put", "global", "enable_non_resizable_multi_window", "1"])
            .output();
        let _ = Command::new(&self.adb_bin)
            .args(["-s", serial, "shell", "settings", "put", "global", "multi_window_menu_in_full_screen", "1"])
            .output();
        Ok(())
    }

    /// Query the device's IP address over ADB
    pub fn get_device_wlan_ip(&self, serial: &str) -> Result<String> {
        let output = Command::new(&self.adb_bin)
            .args(["-s", serial, "shell", "ip", "-f", "inet", "addr", "show", "wlan0"])
            .output()?;

        let text = String::from_utf8_lossy(&output.stdout);
        let re_ip = Regex::new(r"inet\s+([0-9]+\.[0-9]+\.[0-9]+\.[0-9]+)").unwrap();
        if let Some(caps) = re_ip.captures(&text) {
            if let Some(ip) = caps.get(1) {
                return Ok(ip.as_str().to_string());
            }
        }

        Err(anyhow!("Could not detect wlan0 IP address"))
    }

    /// Check if phone screen is locked (Keyguard showing or Dozing)
    pub fn check_is_locked(&self, serial: &str) -> bool {
        if let Ok(output) = Command::new(&self.adb_bin)
            .args(["-s", serial, "shell", "dumpsys", "window"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            if text.contains("isKeyguardShowing=true") || text.contains("mShowingLockscreen=true") {
                return true;
            }
        }
        false
    }
}
