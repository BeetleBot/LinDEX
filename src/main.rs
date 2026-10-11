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

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

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
        configure_modern_theme(&cc.egui_ctx);

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
            status_text: "Ready to connect".to_string(),
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
                    let _ = tx.send(AppEvent::StatusMessage(format!("Device scan failed: {}", e), true));
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
                thread::sleep(std::time::Duration::from_millis(800));
                let _ = adb.connect_wifi(&wlan_ip, 5555);
                let _ = tx.send(AppEvent::StatusMessage(
                    format!("Wi-Fi enabled at {}:5555. You can now unplug USB!", wlan_ip),
                    false,
                ));
            } else {
                let _ = adb.enable_tcpip(Some(&s), 5555);
                let _ = tx.send(AppEvent::StatusMessage(
                    "TCP/IP enabled on port 5555. Connect using phone's Wi-Fi IP.".to_string(),
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
                self.status_text = format!("Failed to stop DeX: {}", e);
                self.status_is_error = true;
            } else {
                self.status_text = "Samsung DeX session ended.".to_string();
                self.status_is_error = false;
            }
            return;
        }

        let serial = match self.selected_device_idx.and_then(|i| self.devices.get(i)) {
            Some(dev) => dev.serial.clone(),
            None => {
                self.status_text = "No Galaxy device selected".to_string();
                self.status_is_error = true;
                return;
            }
        };

        self.config.save();
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

fn toggle_switch(ui: &mut egui::Ui, on: &mut bool, label: &str) {
    ui.horizontal(|ui| {
        let desired_size = Vec2::new(34.0, 18.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
        if response.clicked() {
            *on = !*on;
        }

        if ui.is_rect_visible(rect) {
            let track_bg = if *on {
                Color32::from_rgb(217, 140, 15) // vibrant gold
            } else {
                Color32::from_rgb(42, 32, 62) // muted dark purple
            };
            let track_stroke = if *on {
                egui::Stroke::new(1.0_f32, Color32::from_rgb(250, 204, 21))
            } else {
                egui::Stroke::new(1.0_f32, Color32::from_rgb(72, 54, 105))
            };
            ui.painter().rect(rect, 9.0_f32, track_bg, track_stroke);

            let radius = 6.0_f32;
            let thumb_x = if *on {
                rect.right() - radius - 3.0_f32
            } else {
                rect.left() + radius + 3.0_f32
            };
            let center = egui::pos2(thumb_x, rect.center().y);
            let thumb_color = if *on {
                Color32::from_rgb(255, 252, 235)
            } else {
                Color32::from_rgb(160, 145, 195)
            };
            ui.painter().circle_filled(center, radius, thumb_color);
        }

        let text_color = if *on {
            Color32::from_rgb(250, 245, 255)
        } else {
            Color32::from_rgb(180, 170, 205)
        };
        if ui.add(egui::Label::new(RichText::new(label).size(12.5).color(text_color)).sense(egui::Sense::click())).clicked() {
            *on = !*on;
        }
    });
}

fn configure_modern_theme(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();

    // Deep midnight purple canvas
    visuals.panel_fill = Color32::from_rgb(17, 13, 26);
    visuals.window_fill = Color32::from_rgb(17, 13, 26);
    visuals.extreme_bg_color = Color32::from_rgb(12, 9, 19);

    // Cards / Containers (rich dark violet-slate)
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(26, 20, 40);
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(58, 44, 86));
    visuals.widgets.noninteractive.rounding = egui::Rounding::same(10.0);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(238, 230, 252));

    // Interactive buttons / controls
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(36, 27, 56);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(68, 52, 100));
    visuals.widgets.inactive.rounding = egui::Rounding::same(8.0);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(230, 220, 248));

    // Hovered: gold border highlight
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(48, 36, 75);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(217, 160, 30));
    visuals.widgets.hovered.rounding = egui::Rounding::same(8.0);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(255, 248, 230));

    // Active
    visuals.widgets.active.bg_fill = Color32::from_rgb(60, 44, 92);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(245, 185, 45));
    visuals.widgets.active.rounding = egui::Rounding::same(8.0);

    // Selection (Warm gold)
    visuals.selection.bg_fill = Color32::from_rgb(217, 140, 15);
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, Color32::from_rgb(251, 207, 110));

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.spacing.window_margin = egui::Margin::same(14.0);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
    ctx.set_style(style);
}

impl eframe::App for LinuxDexApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll background events
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

        // --- Top Header Bar ---
        egui::TopBottomPanel::top("top_panel")
            .frame(egui::Frame::none().fill(Color32::from_rgb(18, 21, 28)).inner_margin(egui::Margin::symmetric(14.0, 10.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // LinDex transparent logo
                    ui.add(egui::Image::new(&self.logo_texture).max_height(28.0));
                    ui.add_space(6.0);

                    // Version badge pill
                    egui::Frame::none()
                        .fill(Color32::from_rgb(32, 38, 52))
                        .rounding(10.0)
                        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(52, 62, 84)))
                        .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(format!("v{}", APP_VERSION))
                                    .size(11.0)
                                    .strong()
                                    .color(Color32::from_rgb(250, 204, 21)),
                            );
                        });

                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Samsung DeX for Linux")
                            .size(13.0)
                            .color(Color32::from_rgb(148, 163, 184)),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🔄 Scan Devices").clicked() {
                            self.trigger_refresh_devices();
                        }
                    });
                });
            });

        // --- Bottom Status Bar ---
        egui::TopBottomPanel::bottom("bottom_panel")
            .frame(egui::Frame::none().fill(Color32::from_rgb(15, 17, 23)).inner_margin(egui::Margin::symmetric(14.0, 8.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let dot_color = if self.status_is_error {
                        Color32::from_rgb(239, 68, 68)
                    } else if self.dex.is_running() {
                        Color32::from_rgb(34, 197, 94)
                    } else {
                        Color32::from_rgb(59, 130, 246)
                    };

                    ui.label(RichText::new("●").size(12.0).color(dot_color));
                    ui.label(RichText::new(&self.status_text).size(12.0).color(Color32::from_rgb(226, 232, 240)));

                    if self.dex.is_running() {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                RichText::new("💡 Host Key: Press [Alt] to release mouse to Linux")
                                    .size(11.0)
                                    .color(Color32::from_rgb(148, 163, 184)),
                            );
                        });
                    }
                });
            });

        // --- Central Workspace ---
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(4.0);

                // Phone locked warning
                if self.dex.is_running() && self.is_phone_locked {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(45, 30, 10))
                        .rounding(8.0)
                        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(217, 119, 6)))
                        .inner_margin(egui::Margin::same(12.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("🔒").size(24.0));
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new("Unlock your phone to use Samsung DeX")
                                            .size(14.0)
                                            .strong()
                                            .color(Color32::from_rgb(251, 191, 36)),
                                    );
                                    ui.label(
                                        RichText::new("Android restricts display apps while locked. Swipe or enter your PIN to continue.")
                                            .size(12.0)
                                            .color(Color32::from_rgb(226, 232, 240)),
                                    );
                                });
                            });
                        });
                    ui.add_space(8.0);
                }

                // 1. Devices Card
                egui::Frame::none()
                    .fill(Color32::from_rgb(26, 20, 40))
                    .rounding(12.0)
                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(58, 44, 86)))
                    .inner_margin(egui::Margin::same(16.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("📱 Connected Galaxy Devices")
                                    .size(15.0)
                                    .strong()
                                    .color(Color32::from_rgb(250, 245, 255)),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let guide_text = if self.show_usb_help { "Hide Guide" } else { "Setup Guide" };
                                let guide_btn = egui::Button::new(
                                    RichText::new(guide_text)
                                        .size(11.5)
                                        .color(Color32::from_rgb(245, 190, 55)),
                                )
                                .fill(Color32::from_rgb(38, 28, 58))
                                .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(85, 64, 128)))
                                .rounding(6.0);

                                if ui.add(guide_btn).clicked() {
                                    self.show_usb_help = !self.show_usb_help;
                                }
                            });
                        });
                        ui.add_space(8.0);

                        if self.devices.is_empty() {
                            egui::Frame::none()
                                .fill(Color32::from_rgb(18, 14, 28))
                                .rounding(8.0)
                                .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(46, 35, 70)))
                                .inner_margin(egui::Margin::same(14.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("🔌").size(24.0));
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new("No Galaxy devices detected")
                                                    .size(13.5)
                                                    .strong()
                                                    .color(Color32::from_rgb(240, 230, 255)),
                                            );
                                            ui.label(
                                                RichText::new("Plug in your phone via USB or connect over Wi-Fi to start.")
                                                    .size(12.0)
                                                    .color(Color32::from_rgb(175, 160, 205)),
                                            );
                                        });
                                    });
                                });
                        } else {
                            let mut action_enable_wifi_serial = None;
                            let mut new_selected_idx = self.selected_device_idx;

                            for (idx, dev) in self.devices.iter().enumerate() {
                                let is_selected = self.selected_device_idx == Some(idx);
                                let (badge_icon, badge_text, badge_color) = match &dev.transport {
                                    DeviceTransport::Wifi(_) => ("📶", "Wi-Fi", Color32::from_rgb(245, 190, 55)),
                                    DeviceTransport::Usb => ("🔌", "USB", Color32::from_rgb(74, 222, 128)),
                                };

                                let row_fill = if is_selected {
                                    Color32::from_rgb(44, 32, 68)
                                } else {
                                    Color32::from_rgb(20, 15, 32)
                                };
                                let row_border = if is_selected {
                                    Color32::from_rgb(234, 179, 8)
                                } else {
                                    Color32::from_rgb(48, 36, 74)
                                };

                                egui::Frame::none()
                                    .fill(row_fill)
                                    .rounding(8.0)
                                    .stroke(egui::Stroke::new(if is_selected { 1.5_f32 } else { 1.0_f32 }, row_border))
                                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.radio_value(&mut new_selected_idx, Some(idx), "");
                                            ui.vertical(|ui| {
                                                ui.label(RichText::new(&dev.model).size(14.0).strong().color(Color32::WHITE));
                                                ui.label(RichText::new(&dev.serial).size(11.0).color(Color32::from_rgb(180, 165, 210)));
                                            });

                                            ui.add_space(8.0);
                                            // Transport badge pill with gold / green accents
                                            let (badge_fill, badge_border) = match &dev.transport {
                                                DeviceTransport::Wifi(_) => (Color32::from_rgb(42, 30, 64), Color32::from_rgb(217, 160, 30)),
                                                DeviceTransport::Usb => (Color32::from_rgb(25, 40, 32), Color32::from_rgb(34, 197, 94)),
                                            };
                                            egui::Frame::none()
                                                .fill(badge_fill)
                                                .stroke(egui::Stroke::new(1.0_f32, badge_border))
                                                .rounding(6.0)
                                                .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                                                .show(ui, |ui| {
                                                    ui.label(
                                                        RichText::new(format!("{} {}", badge_icon, badge_text))
                                                            .size(11.0)
                                                            .strong()
                                                            .color(badge_color),
                                                    );
                                                });

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if let DeviceTransport::Usb = &dev.transport {
                                                    let wifi_btn = egui::Button::new(
                                                        RichText::new("⚡ Switch to Wi-Fi")
                                                            .size(11.5)
                                                            .color(Color32::from_rgb(250, 204, 21)),
                                                    )
                                                    .fill(Color32::from_rgb(46, 34, 72))
                                                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(217, 160, 30)))
                                                    .rounding(6.0);

                                                    if ui.add(wifi_btn).clicked() {
                                                        action_enable_wifi_serial = Some(dev.serial.clone());
                                                    }
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(4.0);
                            }

                            self.selected_device_idx = new_selected_idx;
                            if let Some(serial) = action_enable_wifi_serial {
                                self.enable_wifi_from_usb(&serial);
                            }
                        }

                        if self.show_usb_help {
                            ui.add_space(6.0);
                            egui::Frame::none()
                                .fill(Color32::from_rgb(20, 15, 32))
                                .rounding(8.0)
                                .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(72, 54, 105)))
                                .inner_margin(egui::Margin::same(12.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new("📖 Quick Setup Guide").size(13.0).strong().color(Color32::from_rgb(245, 190, 55)));
                                    ui.add_space(3.0);
                                    ui.label("1. Open Settings > About phone > Software information on your Samsung phone.");
                                    ui.label("2. Tap 'Build number' 7 times to unlock Developer options.");
                                    ui.label("3. Go to Settings > Developer options and enable 'USB debugging'.");
                                    ui.label("4. Connect your phone via USB cable and tap 'Always allow' on your phone.");
                                    ui.label("5. Click 'Launch Samsung DeX' below, or 'Switch to Wi-Fi' to go wireless!");
                                });
                        }
                    });

                ui.add_space(10.0);

                // 2. Display Preferences Card
                egui::Frame::none()
                    .fill(Color32::from_rgb(26, 20, 40))
                    .rounding(12.0)
                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(58, 44, 86)))
                    .inner_margin(egui::Margin::same(16.0))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("🖥️ Display & Performance Preferences")
                                .size(15.0)
                                .strong()
                                .color(Color32::from_rgb(250, 245, 255)),
                        );
                        ui.add_space(10.0);

                        // Resolution segmented pills
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Resolution Preset:").size(13.0).color(Color32::from_rgb(210, 195, 235)));
                            ui.add_space(6.0);

                            egui::Frame::none()
                                .fill(Color32::from_rgb(18, 14, 28))
                                .rounding(8.0)
                                .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(48, 36, 72)))
                                .inner_margin(egui::Margin::symmetric(4.0, 4.0))
                                .show(ui, |ui| {
                                    let res_presets = [
                                        (1920, 1080, 200, "1080p FHD"),
                                        (2560, 1440, 220, "1440p QHD"),
                                        (3840, 2160, 280, "4K UHD"),
                                    ];

                                    for (w, h, dpi, label) in res_presets {
                                        let is_active = self.config.width == w && self.config.height == h;
                                        let btn_fill = if is_active {
                                            Color32::from_rgb(217, 140, 15) // Gold pill
                                        } else {
                                            Color32::TRANSPARENT
                                        };
                                        let text_color = if is_active {
                                            Color32::from_rgb(20, 14, 30) // Dark readable text on gold
                                        } else {
                                            Color32::from_rgb(200, 185, 225)
                                        };

                                        let btn = egui::Button::new(
                                            RichText::new(label)
                                                .size(12.0)
                                                .strong()
                                                .color(text_color),
                                        )
                                        .fill(btn_fill)
                                        .rounding(6.0);

                                        if ui.add(btn).clicked() {
                                            self.config.width = w;
                                            self.config.height = h;
                                            self.config.dpi = dpi;
                                        }
                                    }
                                });
                        });

                        ui.add_space(8.0);

                        // DPI and Bitrate sliders
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("DPI Scaling:").size(13.0).color(Color32::from_rgb(210, 195, 235)));
                            ui.add(egui::Slider::new(&mut self.config.dpi, 120..=360).text("DPI"));

                            ui.add_space(16.0);
                            ui.label(RichText::new("Bitrate:").size(13.0).color(Color32::from_rgb(210, 195, 235)));
                            ui.add(egui::Slider::new(&mut self.config.bit_rate_mbps, 8..=50).text("Mbps"));
                        });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(8.0);

                        // Modern Toggle Switches in a clean grid
                        ui.label(RichText::new("Session Options").size(12.0).strong().color(Color32::from_rgb(245, 190, 55)));
                        ui.add_space(6.0);

                        egui::Grid::new("toggle_grid")
                            .num_columns(2)
                            .spacing([24.0, 10.0])
                            .show(ui, |ui| {
                                toggle_switch(ui, &mut self.config.use_h265, "H.265 (HEVC Acceleration)");
                                toggle_switch(ui, &mut self.config.forward_audio, "Forward Phone Audio");
                                ui.end_row();

                                toggle_switch(ui, &mut self.config.turn_screen_off, "Turn Phone Screen Off");
                                toggle_switch(ui, &mut self.config.fullscreen, "Start Fullscreen");
                                ui.end_row();

                                toggle_switch(ui, &mut self.config.capture_system_keys, "Capture Super / Windows Key");
                                ui.label("");
                                ui.end_row();
                            });
                    });

                ui.add_space(14.0);

                // 3. Prominent Action Button
                ui.vertical_centered(|ui| {
                    let is_active = self.dex.is_running();
                    let (btn_text, btn_fill, text_color, btn_stroke) = if is_active {
                        (
                            "🛑  Stop Samsung DeX Session",
                            Color32::from_rgb(190, 30, 60),
                            Color32::WHITE,
                            egui::Stroke::new(1.0_f32, Color32::from_rgb(244, 63, 94)),
                        )
                    } else {
                        (
                            "✨  Launch Samsung DeX",
                            Color32::from_rgb(217, 145, 18), // Rich Gold
                            Color32::from_rgb(20, 14, 30),   // Obsidian bold text on gold
                            egui::Stroke::new(1.5_f32, Color32::from_rgb(250, 204, 21)), // Bright gold stroke
                        )
                    };

                    let button = egui::Button::new(
                        RichText::new(btn_text)
                            .size(16.0)
                            .strong()
                            .color(text_color),
                    )
                    .fill(btn_fill)
                    .stroke(btn_stroke)
                    .rounding(12.0)
                    .min_size(Vec2::new(340.0, 50.0));

                    if ui.add(button).clicked() {
                        self.launch_dex();
                    }

                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("💡 Host Key: Press [Alt] anytime to release cursor/keyboard back to Linux")
                            .size(11.5)
                            .color(Color32::from_rgb(210, 180, 120)),
                    );
                });

                ui.add_space(10.0);
            });
        });

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
        .with_inner_size([740.0, 580.0])
        .with_min_inner_size([660.0, 500.0])
        .with_title("LinDex - Samsung DeX for Linux");

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "lindex",
        native_options,
        Box::new(|cc| Ok(Box::new(LinuxDexApp::new(cc)))),
    )
}
