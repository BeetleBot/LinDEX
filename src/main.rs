mod adb;
mod config;
mod dex;

use adb::{AdbDevice, AdbManager, DeviceTransport};
use config::AppConfig;
use dex::DexSession;
use eframe::egui::{self, Color32, RichText, Vec2};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

enum AppEvent {
    DevicesUpdated(Vec<AdbDevice>),
    StatusMessage(String, bool), // (message, is_error)
    DeviceLockStatus(bool),      // true if phone is locked
}

struct LinuxDexApp {
    config: AppConfig,
    adb: Arc<AdbManager>,
    dex: Arc<DexSession>,

    // Discovered devices & selection
    devices: Vec<AdbDevice>,
    selected_device_idx: Option<usize>,
    is_phone_locked: bool,
    show_usb_help: bool,
    logo_texture: egui::TextureHandle,

    // Status banner
    status_text: String,
    status_is_error: bool,

    // Channels
    event_tx: Sender<AppEvent>,
    event_rx: Receiver<AppEvent>,
    is_refreshing: bool,
}

impl LinuxDexApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Set dark theme by default
        cc.egui_ctx.set_visuals(egui::Visuals::dark());

        let logo_texture = {
            let bytes = include_bytes!("../assets/logo.png");
            let img = image::load_from_memory(bytes).expect("Failed to load logo image");
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            cc.egui_ctx.load_texture("lindex_logo", color_image, egui::TextureOptions::LINEAR)
        };

        let config = AppConfig::load();
        let adb = Arc::new(AdbManager::new());
        let dex = Arc::new(DexSession::new());
        let (event_tx, event_rx) = channel();

        let app = Self {
            config,
            adb,
            dex,
            devices: Vec::new(),
            selected_device_idx: None,
            is_phone_locked: false,
            show_usb_help: false,
            logo_texture,
            status_text: "Ready".to_string(),
            status_is_error: false,
            event_tx,
            event_rx,
            is_refreshing: false,
        };

        app.trigger_refresh_devices();
        app.start_lock_status_monitor();
        app
    }

    fn start_lock_status_monitor(&self) {
        let adb = Arc::clone(&self.adb);
        let dex = Arc::clone(&self.dex);
        let tx = self.event_tx.clone();

        thread::spawn(move || {
            loop {
                thread::sleep(std::time::Duration::from_millis(1500));
                if dex.is_running() {
                    if let Ok(devs) = adb.list_devices() {
                        if let Some(dev) = devs.first() {
                            let locked = adb.check_is_locked(&dev.serial);
                            let _ = tx.send(AppEvent::DeviceLockStatus(locked));
                        }
                    }
                }
            }
        });
    }

    fn trigger_refresh_devices(&self) {
        let adb = Arc::clone(&self.adb);
        let tx = self.event_tx.clone();

        thread::spawn(move || {
            match adb.list_devices() {
                Ok(devs) => {
                    let _ = tx.send(AppEvent::DevicesUpdated(devs));
                }
                Err(e) => {
                    let _ = tx.send(AppEvent::StatusMessage(format!("ADB scan failed: {}", e), true));
                }
            }
        });
    }

    fn enable_wifi_from_usb(&mut self, serial: &str) {
        let adb = Arc::clone(&self.adb);
        let tx = self.event_tx.clone();
        let s = serial.to_string();
        self.status_text = "Enabling Wi-Fi mode via USB...".to_string();

        thread::spawn(move || {
            if let Ok(wlan_ip) = adb.get_device_wlan_ip(&s) {
                let _ = adb.enable_tcpip(Some(&s), 5555);
                thread::sleep(std::time::Duration::from_millis(1000));
                let _ = adb.connect_wifi(&wlan_ip, 5555);
                let _ = tx.send(AppEvent::StatusMessage(
                    format!("Wi-Fi mode enabled ({}:5555)! You can now unplug the USB cable.", wlan_ip),
                    false,
                ));
            } else {
                let _ = adb.enable_tcpip(Some(&s), 5555);
                let _ = tx.send(AppEvent::StatusMessage(
                    "TCP/IP enabled on port 5555.".to_string(),
                    false,
                ));
            }
            if let Ok(devs) = adb.list_devices() {
                let _ = tx.send(AppEvent::DevicesUpdated(devs));
            }
        });
    }

    fn launch_dex(&mut self) {
        if self.dex.is_running() {
            if let Err(e) = self.dex.stop() {
                self.status_text = format!("Failed to stop: {}", e);
                self.status_is_error = true;
            } else {
                self.status_text = "DeX session ended.".to_string();
                self.status_is_error = false;
            }
            return;
        }

        let serial = match self.selected_device_idx.and_then(|i| self.devices.get(i)) {
            Some(dev) => dev.serial.clone(),
            None => {
                self.status_text = "Please select or connect a device first.".to_string();
                self.status_is_error = true;
                return;
            }
        };

        self.config.save();
        let _ = self.adb.enable_multi_window_desktop(&serial);
        match self.dex.launch(&serial, &self.config) {
            Ok(_) => {
                self.status_text = format!("Samsung DeX running on {}", serial);
                self.status_is_error = false;
            }
            Err(e) => {
                self.status_text = format!("Failed to launch DeX: {}", e);
                self.status_is_error = true;
            }
        }
    }
}

impl eframe::App for LinuxDexApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll asynchronous background events
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                AppEvent::DevicesUpdated(devs) => {
                    self.devices = devs;
                    if self.selected_device_idx.is_none() && !self.devices.is_empty() {
                        self.selected_device_idx = Some(0);
                    }
                }
                AppEvent::StatusMessage(msg, is_error) => {
                    self.status_text = msg;
                    self.status_is_error = is_error;
                }
                AppEvent::DeviceLockStatus(locked) => {
                    self.is_phone_locked = locked;
                }
            }
        }

        // Header Panel
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.add(egui::Image::new(&self.logo_texture).max_height(32.0));
                ui.add_space(4.0);
                ui.label(
                    RichText::new("DeX for Linux")
                        .size(13.0)
                        .italics()
                        .color(Color32::from_rgb(180, 180, 190)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Refresh").clicked() {
                        self.trigger_refresh_devices();
                    }
                });
            });
            ui.add_space(8.0);
        });

        // Bottom Status Panel
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let color = if self.status_is_error {
                    Color32::from_rgb(255, 100, 100)
                } else {
                    Color32::from_rgb(100, 220, 120)
                };
                ui.label(RichText::new(&self.status_text).color(color));

                if self.dex.is_running() {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("Host Key: Press Alt to release cursor").size(11.0).color(Color32::LIGHT_GRAY));
                    });
                }
            });
            ui.add_space(6.0);
        });

        // Central Panel
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(6.0);

                // Warn if phone is locked
                if self.dex.is_running() && self.is_phone_locked {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(45, 30, 10))
                        .rounding(6.0)
                        .stroke(egui::Stroke::new(1.5_f32, Color32::from_rgb(230, 150, 30)))
                        .inner_margin(egui::Margin::same(10.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("⚠").size(20.0).color(Color32::from_rgb(255, 200, 80)));
                                ui.vertical(|ui| {
                                    ui.label(RichText::new("Unlock your phone to use Samsung DeX").size(14.0).strong().color(Color32::from_rgb(255, 200, 80)));
                                    ui.label(RichText::new("DeX input is restricted while the phone is locked. Unlock the phone to continue.").size(12.0).color(Color32::LIGHT_GRAY));
                                });
                            });
                        });
                    ui.add_space(8.0);
                }

                // Device list
                ui.group(|ui| {
                    ui.heading("Connected Devices");
                    ui.add_space(4.0);

                    if self.devices.is_empty() {
                        ui.label(RichText::new("No devices found. Plug in USB or connect via Wi-Fi.").italics().color(Color32::GRAY));
                    } else {
                        let mut action_enable_wifi_serial = None;
                        let mut new_selected_idx = self.selected_device_idx;

                        for (idx, dev) in self.devices.iter().enumerate() {
                            let is_selected = self.selected_device_idx == Some(idx);
                            let transport_tag = match &dev.transport {
                                DeviceTransport::Wifi(_) => "Wi-Fi",
                                DeviceTransport::Usb => "USB",
                            };

                            ui.horizontal(|ui| {
                                let label = format!("{} ({}) - {}", dev.model, dev.serial, transport_tag);
                                if ui.selectable_label(is_selected, label).clicked() {
                                    new_selected_idx = Some(idx);
                                }

                                if let DeviceTransport::Usb = &dev.transport {
                                    if ui.button("Switch to Wi-Fi").clicked() {
                                        action_enable_wifi_serial = Some(dev.serial.clone());
                                    }
                                }
                            });
                        }

                        self.selected_device_idx = new_selected_idx;
                        if let Some(serial) = action_enable_wifi_serial {
                            self.enable_wifi_from_usb(&serial);
                        }
                    }
                });

                ui.add_space(8.0);

                // Connection guide
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Connection");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let guide_btn_text = if self.show_usb_help { "Hide Guide" } else { "Setup Guide" };
                            if ui.button(guide_btn_text).clicked() {
                                self.show_usb_help = !self.show_usb_help;
                            }
                        });
                    });
                    ui.add_space(2.0);

                    if self.show_usb_help {
                        egui::Frame::none()
                            .fill(Color32::from_rgb(25, 30, 40))
                            .rounding(4.0)
                            .inner_margin(egui::Margin::same(8.0))
                            .show(ui, |ui| {
                                ui.label(RichText::new("To enable USB Debugging:").strong().color(Color32::from_rgb(100, 180, 255)));
                                ui.label("1. Open Settings > About phone > Software information.");
                                ui.label("2. Tap 'Build number' 7 times to enable Developer options.");
                                ui.label("3. Go to Settings > Developer options and enable 'USB debugging'.");
                                ui.label("4. Connect via USB and accept the debugging prompt on your phone.");
                            });
                        ui.add_space(4.0);
                    }

                    ui.label(RichText::new("Modes:").strong());
                    ui.label("• Wired: Plug in via USB and click Launch.");
                    ui.label("• Wi-Fi: Connect via USB once, click 'Switch to Wi-Fi', then unplug the cable.");
                });

                ui.add_space(8.0);

                // Display settings
                ui.group(|ui| {
                    ui.heading("Display Preferences");
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Resolution:");
                        if ui.selectable_label(self.config.width == 1920 && self.config.height == 1080, "1080p FHD").clicked() {
                            self.config.width = 1920;
                            self.config.height = 1080;
                            self.config.dpi = 200;
                        }
                        if ui.selectable_label(self.config.width == 2560 && self.config.height == 1440, "1440p QHD").clicked() {
                            self.config.width = 2560;
                            self.config.height = 1440;
                            self.config.dpi = 220;
                        }
                        if ui.selectable_label(self.config.width == 3840 && self.config.height == 2160, "4K UHD").clicked() {
                            self.config.width = 3840;
                            self.config.height = 2160;
                            self.config.dpi = 280;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("DPI Scaling:");
                        ui.add(egui::Slider::new(&mut self.config.dpi, 120..=360).text("DPI"));
                        ui.label("Bitrate:");
                        ui.add(egui::Slider::new(&mut self.config.bit_rate_mbps, 8..=50).text("Mbps"));
                    });

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.config.use_h265, "H.265 (HEVC)");
                        ui.checkbox(&mut self.config.forward_audio, "Audio forwarding");
                        ui.checkbox(&mut self.config.turn_screen_off, "Turn off phone screen");
                        ui.checkbox(&mut self.config.fullscreen, "Fullscreen");
                    });

                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.config.capture_system_keys, "Capture system keys (Super / Windows key)");
                    });
                    ui.label(RichText::new("Press Left-Alt to release mouse cursor back to host.").size(11.0).color(Color32::GRAY));
                });

                ui.add_space(12.0);

                // Action button
                ui.vertical_centered(|ui| {
                    let is_active = self.dex.is_running();
                    let btn_text = if is_active {
                        "Stop DeX"
                    } else {
                        "Launch Samsung DeX"
                    };

                    let btn_color = if is_active {
                        Color32::from_rgb(200, 50, 50)
                    } else {
                        Color32::from_rgb(0, 120, 220)
                    };

                    let button = egui::Button::new(
                        RichText::new(btn_text)
                            .size(17.0)
                            .color(Color32::WHITE)
                            .strong()
                    )
                    .fill(btn_color)
                    .min_size(Vec2::new(320.0, 44.0));

                    if ui.add(button).clicked() {
                        self.launch_dex();
                    }
                });

                ui.add_space(10.0);
            });
        });

        // Continuous redraw when refreshing
        if self.is_refreshing {
            ctx.request_repaint();
        }
    }
}

fn load_app_icon() -> Option<egui::IconData> {
    let icon_bytes = include_bytes!("../assets/icon_256.png");
    if let Ok(img) = image::load_from_memory(icon_bytes) {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Some(egui::IconData {
            rgba: rgba.into_raw(),
            width,
            height,
        })
    } else {
        None
    }
}

fn main() -> eframe::Result<()> {
    tracing_subscriber::fmt::init();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([720.0, 560.0])
        .with_min_inner_size([640.0, 480.0])
        .with_title("LinDex - Samsung DeX for Linux");

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "LinDex",
        native_options,
        Box::new(|cc| Ok(Box::new(LinuxDexApp::new(cc)))),
    )
}
