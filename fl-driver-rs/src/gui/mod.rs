pub mod theme;
pub mod keyboard;
pub mod color_picker;
pub mod macro_view;

use std::sync::{Arc, Mutex};
use eframe::egui;
use egui::{Color32, Pos2, RichText, Stroke, Vec2};

use crate::backend::{KeyboardBackend, KeyboardInfo};
use crate::protocol::{LightMode, PollingRate};
use theme::*;
use keyboard::{render_keyboard_canvas, KeyboardDisplayMode, get_tkl_87_layout, key_id_to_vk};
use color_picker::{show_color_picker_popup, ColorPickerState};
use macro_view::{render_macro_view, MacroViewState, play_macro_sequence, MacroAction};
use std::time::{Duration, Instant};

pub fn run_gui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 720.0])
            .with_min_inner_size([980.0, 640.0])
            .with_title("FL-ESPORTS MK870 Keyboard Driver (Clean & Offline Edition)"),
        ..Default::default()
    };

    eframe::run_native(
        "fl-driver-rs",
        options,
        Box::new(|_cc| Ok(Box::new(KeyboardApp::new()))),
    )
}

/// True once a slider value should be written to the keyboard: on release after a drag,
/// or on a click/keyboard change. Over 2.4G every write is a relayed round-trip.
fn slider_committed(resp: &egui::Response) -> bool {
    resp.drag_stopped() || (resp.changed() && !resp.dragged())
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum NavTab {
    Main,
    FnSetting,
    Macro,
    Light,
    Settings,
    About,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum KeySubTab {
    KeySetting,
    OtherSetting,
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub enum LightSubTab {
    Light,
    LightEdit,
    MyWork,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum AssignType {
    Combination,
    Macro,
    Media,
    Mouse,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum MacroTriggerMode {
    Loop,
    Auto,
    Press,
}

#[derive(Clone, Debug, PartialEq)]
pub enum KeyAssignment {
    Default,
    Forbidden,
    Combination { ctrl: bool, alt: bool, shift: bool, win: bool, key: String },
    Macro { name: String, mode: MacroTriggerMode, loops: u32 },
    Media(String),
    Mouse(String),
}

#[derive(Clone)]
pub struct ActiveMacroBinding {
    pub trigger_vk: u8,
    pub actions: Vec<MacroAction>,
    pub mode: MacroTriggerMode,
    pub loops: u32,
}

pub struct KeyboardApp {
    backend: Option<Arc<Mutex<KeyboardBackend>>>,
    info: Option<KeyboardInfo>,
    error_message: Option<String>,
    status_message: Option<String>,

    current_nav: NavTab,
    key_sub_tab: KeySubTab,
    light_sub_tab: LightSubTab,

    // Keyboard state
    selected_key_id: Option<usize>,
    redefined_keys: Vec<bool>,
    fn_redefined_keys: Vec<bool>,

    // Key Mapping bindings
    key_combo_ctrl: bool,
    key_combo_alt: bool,
    key_combo_shift: bool,
    key_combo_win: bool,
    key_combo_char: String,
    selected_macro_name: String,
    selected_media_action: Option<String>,
    selected_mouse_action: Option<String>,
    active_assign_type: AssignType,
    macro_trigger_mode: MacroTriggerMode,
    macro_loop_count: u32,
    last_selected_key_id: Option<usize>,
    key_assignments: Vec<KeyAssignment>,
    fn_key_assignments: Vec<KeyAssignment>,
    show_tutorial: bool,
    active_macro_bindings: Arc<Mutex<Vec<ActiveMacroBinding>>>,

    // Macro state
    macro_state: MacroViewState,

    // Light presets
    selected_mode: LightMode,
    brightness: u8,
    speed: u8,
    rainbow: bool,

    // Light Edit state (per-key)
    key_colors: Vec<[u8; 3]>,
    active_paint_color: [u8; 3],
    color_picker_state: ColorPickerState,
    light_layer: u8,
    light_layers: crate::storage::LightLayerStorage,

    // Other Setting / Performance state
    polling_rate: PollingRate,
    debounce_ms: u8,
    sleep_24_min: u16,
    sleep_bt_min: u16,

    // Accessibility & Settings
    ui_scale: f32,
    font_size_delta: f32,

    // Hot-plug detection & Wireless Mode state
    last_device_scan: Instant,
    last_rf_poll: Instant,
    rf_online: bool,
}

impl KeyboardApp {
    pub fn new() -> Self {
        let app_settings = crate::storage::load_settings();
        let mut app = Self {
            backend: None,
            info: None,
            error_message: None,
            status_message: None,

            current_nav: NavTab::Main,
            key_sub_tab: KeySubTab::KeySetting,
            light_sub_tab: LightSubTab::Light,

            selected_key_id: None,
            redefined_keys: vec![false; 87],
            fn_redefined_keys: vec![false; 87],

            key_combo_ctrl: false,
            key_combo_alt: false,
            key_combo_shift: false,
            key_combo_win: false,
            key_combo_char: String::new(),
            selected_macro_name: "Macro 1".to_string(),
            selected_media_action: None,
            selected_mouse_action: None,
            active_assign_type: AssignType::Combination,
            macro_trigger_mode: MacroTriggerMode::Loop,
            macro_loop_count: 1,
            last_selected_key_id: None,
            key_assignments: vec![KeyAssignment::Default; 87],
            fn_key_assignments: vec![KeyAssignment::Default; 87],
            show_tutorial: false,
            active_macro_bindings: {
                let bindings: Arc<Mutex<Vec<ActiveMacroBinding>>> = Arc::new(Mutex::new(Vec::new()));
                let b_clone = Arc::clone(&bindings);
                std::thread::spawn(move || {
                    #[link(name = "user32")]
                    extern "system" {
                        fn GetAsyncKeyState(vKey: i32) -> i16;
                    }
                    use std::sync::atomic::{AtomicBool, Ordering};
                    let mut was_down = [false; 256];
                    let mut auto_active = [false; 256];
                    let is_executing = Arc::new(AtomicBool::new(false));

                    loop {
                        std::thread::sleep(Duration::from_millis(8));
                        let list = {
                            if let Ok(g) = b_clone.lock() {
                                g.clone()
                            } else {
                                Vec::new()
                            }
                        };
                        if list.is_empty() {
                            continue;
                        }
                        for b in list {
                            let vk = b.trigger_vk as i32;
                            let is_down = unsafe { (GetAsyncKeyState(vk) as u16 & 0x8000) != 0 };

                            match b.mode {
                                MacroTriggerMode::Loop => {
                                    if is_down && !was_down[b.trigger_vk as usize] {
                                        was_down[b.trigger_vk as usize] = true;
                                        if !is_executing.load(Ordering::Relaxed) {
                                            let is_exec = Arc::clone(&is_executing);
                                            let actions = b.actions.clone();
                                            let loops = b.loops;
                                            std::thread::spawn(move || {
                                                is_exec.store(true, Ordering::Relaxed);
                                                for _ in 0..loops {
                                                    play_macro_sequence(&actions);
                                                }
                                                is_exec.store(false, Ordering::Relaxed);
                                            });
                                        }
                                    } else if !is_down {
                                        was_down[b.trigger_vk as usize] = false;
                                    }
                                }
                                MacroTriggerMode::Press => {
                                    if is_down && !is_executing.load(Ordering::Relaxed) {
                                        let is_exec = Arc::clone(&is_executing);
                                        let actions = b.actions.clone();
                                        std::thread::spawn(move || {
                                            is_exec.store(true, Ordering::Relaxed);
                                            play_macro_sequence(&actions);
                                            is_exec.store(false, Ordering::Relaxed);
                                        });
                                    }
                                }
                                MacroTriggerMode::Auto => {
                                    if is_down && !was_down[b.trigger_vk as usize] {
                                        was_down[b.trigger_vk as usize] = true;
                                        auto_active[b.trigger_vk as usize] = !auto_active[b.trigger_vk as usize];
                                    } else if !is_down {
                                        was_down[b.trigger_vk as usize] = false;
                                    }
                                    if auto_active[b.trigger_vk as usize] && !is_executing.load(Ordering::Relaxed) {
                                        let is_exec = Arc::clone(&is_executing);
                                        let actions = b.actions.clone();
                                        std::thread::spawn(move || {
                                            is_exec.store(true, Ordering::Relaxed);
                                            play_macro_sequence(&actions);
                                            is_exec.store(false, Ordering::Relaxed);
                                        });
                                    }
                                }
                            }
                        }
                    }
                });
                bindings
            },

            macro_state: MacroViewState::default(),

            selected_mode: LightMode::Wave,
            brightness: 4,
            speed: 3,
            rainbow: true,

            // Load persistent custom lighting layers
            key_colors: {
                let layers = crate::storage::load_light_layers();
                if layers.layer_1.iter().any(|c| *c != [0, 0, 0]) {
                    layers.layer_1.clone()
                } else {
                    let mut colors = vec![[0, 0, 0]; 87];
                    if colors.len() > 50 {
                        colors[50] = [255, 255, 255];
                    }
                    colors
                }
            },
            active_paint_color: [255, 255, 255],
            color_picker_state: ColorPickerState::default(),
            light_layer: 1,
            light_layers: crate::storage::load_light_layers(),

            polling_rate: PollingRate::Hz1000,
            debounce_ms: 4,
            sleep_24_min: 2,
            sleep_bt_min: 2,

            ui_scale: app_settings.ui_scale,
            font_size_delta: app_settings.font_size_delta,

            last_device_scan: Instant::now(),
            last_rf_poll: Instant::now(),
            rf_online: false,
        };

        app.init_backend();
        app
    }

    pub fn is_wireless_mode(&self) -> bool {
        self.info.as_ref().map(|i| i.is_wireless).unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn is_connected(&self) -> bool {
        self.info.is_some()
    }

    fn init_backend(&mut self) {
        match KeyboardBackend::connect() {
            Ok(b) => {
                let is_wl = b.is_wireless;
                if let Ok(info) = b.get_keyboard_info() {
                    self.info = Some(info);
                }
                if let Ok(led) = b.get_rgb() {
                    self.selected_mode = led.mode;
                    self.brightness = led.brightness;
                    self.speed = led.speed;
                    self.rainbow = led.dazzle;
                }
                if let Ok(rate) = b.get_polling_rate() {
                    self.polling_rate = rate;
                }
                if let Ok(sleep) = b.get_sleep() {
                    self.sleep_24_min = (sleep.time_24 / 60).max(1);
                    self.sleep_bt_min = (sleep.time_bt / 60).max(1);
                }
                let relay = b.rf_relay;
                self.rf_online = if relay { b.rf_status().map(|s| s.online).unwrap_or(false) } else { true };
                self.backend = Some(Arc::new(Mutex::new(b)));
                self.error_message = None;
                if !is_wl {
                    self.status_message = Some("Connected via USB Cable.".to_string());
                } else if !relay {
                    self.status_message = None;
                    self.error_message = Some("Connected via 2.4G dongle. This receiver only carries keystrokes; connect the USB cable to change keyboard settings.".to_string());
                } else if self.rf_online {
                    self.status_message = Some("Connected via 2.4G Wireless Dongle.".to_string());
                } else {
                    self.status_message = None;
                    self.error_message = Some("2.4G dongle found but the keyboard is asleep. Press any key on the MK870 to wake it.".to_string());
                }
            }
            Err(e) => {
                self.error_message = Some(format!("Device not detected: {}. (Switch to 2.4G or USB cable)", e));
            }
        }
    }

    fn apply_rgb(&mut self) {
        if let Some(backend_arc) = &self.backend {
            if let Ok(backend) = backend_arc.lock() {
                let col = self.active_paint_color;
                match backend.set_rgb(self.selected_mode, self.brightness, self.speed, col, self.rainbow) {
                    Ok(_) => {
                        self.status_message = Some(format!("Applied RGB: {}", self.selected_mode.name()));
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to set RGB: {}", e));
                    }
                }
            }
        }
    }

    fn apply_per_key_rgb(&mut self) {
        if let Some(backend_arc) = &self.backend {
            if let Ok(backend) = backend_arc.lock() {
                match backend.set_per_key_rgb(&self.key_colors, self.light_layer, self.brightness) {
                    Ok(_) => {
                        self.status_message = Some(format!("✔ Custom per-key illumination successfully applied to Layer {}!", self.light_layer));
                        self.error_message = None;
                    }
                    Err(e) => {
                        self.error_message = Some(format!("Failed to upload custom lights: {}", e));
                    }
                }
            }
        } else {
            self.error_message = Some("Keyboard not detected. Connect your MK870 via USB cable or 2.4G dongle to upload custom lights.".to_string());
        }
    }

    fn sync_active_macro_bindings(&self) {
        let mut bindings = Vec::new();
        // 1. Primary layer (Main tab)
        for (id, is_redef) in self.redefined_keys.iter().enumerate() {
            if *is_redef {
                if let Some(KeyAssignment::Macro { name, mode, loops }) = self.key_assignments.get(id) {
                    if let Some(vk) = key_id_to_vk(id) {
                        if let Some(m) = self.macro_state.macros.iter().find(|m| &m.name == name) {
                            bindings.push(ActiveMacroBinding {
                                trigger_vk: vk,
                                actions: m.actions.clone(),
                                mode: *mode,
                                loops: *loops,
                            });
                        }
                    }
                }
            }
        }
        // 2. Fn layer fallback (also register macros configured under FnSetting so they trigger!)
        for (id, is_redef) in self.fn_redefined_keys.iter().enumerate() {
            if *is_redef && !self.redefined_keys[id] {
                if let Some(KeyAssignment::Macro { name, mode, loops }) = self.fn_key_assignments.get(id) {
                    if let Some(vk) = key_id_to_vk(id) {
                        if let Some(m) = self.macro_state.macros.iter().find(|m| &m.name == name) {
                            bindings.push(ActiveMacroBinding {
                                trigger_vk: vk,
                                actions: m.actions.clone(),
                                mode: *mode,
                                loops: *loops,
                            });
                        }
                    }
                }
            }
        }
        if let Ok(mut guard) = self.active_macro_bindings.lock() {
            *guard = bindings;
        }
    }

    /// Runs a hardware command and reports the outcome in the footer.
    fn run_device_command(&mut self, success: String, cmd: impl FnOnce(&KeyboardBackend) -> Result<(), String>) {
        let result = match &self.backend {
            Some(backend_arc) => match backend_arc.lock() {
                Ok(backend) => cmd(&backend),
                Err(_) => return,
            },
            None => return,
        };
        match result {
            Ok(()) => {
                self.status_message = Some(success);
                self.error_message = None;
            }
            Err(e) => {
                self.status_message = None;
                self.error_message = Some(e);
            }
        }
    }

    fn apply_polling_rate(&mut self) {
        let rate = self.polling_rate;
        self.run_device_command(format!("Polling rate updated to {} Hz", rate.to_hz()), |b| b.set_polling_rate(rate));
    }

    fn apply_debounce(&mut self) {
        let ms = self.debounce_ms;
        self.run_device_command(format!("Debounce filter set to {} ms", ms), |b| b.set_debounce(ms));
    }

    fn apply_sleep(&mut self) {
        let s24 = self.sleep_24_min * 60;
        let sbt = self.sleep_bt_min * 60;
        self.run_device_command("Sleep timeout saved to device".to_string(), |b| b.set_sleep(sbt, s24, sbt * 14, s24 * 14));
    }

    fn sync_clock(&mut self) {
        self.run_device_command("Synchronized onboard clock with system time".to_string(), |b| b.sync_clock());
    }

    fn factory_reset(&mut self) {
        self.run_device_command("Keyboard factory reset complete".to_string(), |b| b.factory_reset());
    }

    /// Refreshes battery level and awake/asleep state from the 2.4G receiver.
    fn poll_rf_status(&mut self) {
        let status = match &self.backend {
            Some(backend_arc) => match backend_arc.lock() {
                Ok(backend) if backend.rf_relay => backend.rf_status(),
                _ => return,
            },
            None => return,
        };
        let Ok(status) = status else { return };
        let woke_up = status.online && !self.rf_online;
        self.rf_online = status.online;
        if let Some(info) = &mut self.info {
            if status.online {
                info.battery_percent = Some(status.battery.min(100));
            }
        }
        if woke_up {
            // Settings could not be read while the keyboard slept; load them now.
            self.init_backend();
        }
    }
}

impl eframe::App for KeyboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.set_zoom_factor(self.ui_scale);
        apply_theme(ctx, self.font_size_delta);

        // Auto-detect hotplugging of USB cable / 2.4G dongle every 1.5s
        ctx.request_repaint_after(Duration::from_millis(1500));
        if self.last_device_scan.elapsed() >= Duration::from_millis(1500) {
            self.last_device_scan = Instant::now();
            match KeyboardBackend::find_hid_device_native() {
                Ok((path, is_wireless)) => {
                    let needs_reload = match &self.info {
                        Some(cur) => cur.is_wireless != is_wireless || cur.path != path,
                        None => true,
                    };
                    if needs_reload {
                        self.init_backend();
                    }
                }
                Err(_) => {
                    if self.info.is_some() {
                        self.backend = None;
                        self.info = None;
                        self.error_message = Some("Device disconnected. Connect MK870 via USB cable or 2.4G dongle.".to_string());
                    }
                }
            }
        }
        if self.is_wireless_mode() && self.last_rf_poll.elapsed() >= Duration::from_secs(5) {
            self.last_rf_poll = Instant::now();
            self.poll_rf_status();
        }

        // TOP HEADER BAR (Logo replacement, Tabs, Window Controls, NO login)
        egui::TopBottomPanel::top("header_panel")
            .frame(egui::Frame::none().fill(Color32::from_rgb(22, 32, 37)).inner_margin(8.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Minimalist clean keyboard badge (replaces snake logo)
                    ui.add_space(8.0);
                    let (logo_rect, _) = ui.allocate_exact_size(Vec2::new(34.0, 34.0), egui::Sense::hover());
                    ui.painter().rect_filled(logo_rect, 6.0, Color32::from_rgb(28, 44, 52));
                    ui.painter().rect_stroke(logo_rect, 6.0, Stroke::new(1.5_f32, COLOR_ACCENT_CYAN));
                    ui.painter().text(
                        logo_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "⌨",
                        egui::FontId::proportional(20.0),
                        COLOR_ACCENT_CYAN,
                    );
                    ui.add_space(16.0);

                    // Navigation Tabs (Main, FnSetting, Macro, Light, Settings, About)
                    let tabs = [
                        (NavTab::Main, "Main"),
                        (NavTab::FnSetting, "FnSetting"),
                        (NavTab::Macro, "Macro"),
                        (NavTab::Light, "Light"),
                        (NavTab::Settings, "Settings"),
                        (NavTab::About, "About"),
                    ];

                    for (tab, label) in tabs {
                        let is_active = self.current_nav == tab;
                        let text = RichText::new(label)
                            .size(15.0)
                            .color(if is_active { Color32::WHITE } else { COLOR_TEXT_MUTED });
                        
                        let (rect, resp) = ui.allocate_exact_size(Vec2::new(75.0, 32.0), egui::Sense::click());
                        if resp.clicked() {
                            self.current_nav = tab;
                        }
                        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text.text(), egui::FontId::proportional(15.0), if is_active { Color32::WHITE } else { COLOR_TEXT_MUTED });

                        if is_active {
                            let line_y = rect.bottom() + 2.0;
                            ui.painter().line_segment(
                                [Pos2::new(rect.left() + 8.0, line_y), Pos2::new(rect.right() - 8.0, line_y)],
                                Stroke::new(2.5_f32, COLOR_ACCENT_CYAN),
                            );
                        }
                    }

                    // Right header: Connection status badge and USB action button
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        if ui.button(RichText::new("🔄 Refresh").size(12.0)).on_hover_text("Refresh device detection (e.g. after plugging in USB cable)").clicked() {
                            self.init_backend();
                        }
                        if let Some(info) = &self.info {
                            if info.is_wireless {
                                if !self.rf_online {
                                    ui.label(RichText::new("💤 Asleep").color(Color32::from_rgb(255, 200, 80)).strong())
                                        .on_hover_text("The keyboard sleeps after idling. Press any key on the MK870 to wake it.");
                                    ui.add_space(4.0);
                                } else if let Some(bat) = info.battery_percent {
                                    let (bat_col, bat_desc) = if bat > 70 {
                                        (Color32::from_rgb(80, 220, 120), "Good")
                                    } else if bat > 35 {
                                        (Color32::from_rgb(255, 200, 80), "Normal")
                                    } else {
                                        (Color32::from_rgb(255, 80, 80), "Low")
                                    };
                                    let bat_label = ui.label(RichText::new(format!("🔋 {}%", bat)).color(bat_col).strong());
                                    bat_label.on_hover_text(format!("Battery Level: {}% ({}), reported by the 2.4G receiver", bat, bat_desc));
                                    ui.add_space(4.0);
                                }
                                if info.config_supported {
                                    ui.label(RichText::new("● 2.4G Wireless Mode").color(COLOR_ACCENT_CYAN).strong());
                                } else {
                                    ui.label(RichText::new("● 2.4G Wireless (keys only)").color(COLOR_ACCENT_CYAN).strong())
                                        .on_hover_text("This receiver forwards keystrokes but not configuration commands.\nConnect the USB cable to change lighting and other settings.");
                                }
                            } else {
                                ui.label(RichText::new("● USB Wired Cable").color(Color32::from_rgb(100, 220, 255)).strong());
                            }
                        } else {
                            ui.label(RichText::new("○ Disconnected").color(Color32::from_rgb(255, 100, 100)));
                        }
                    });
                });
            });

        // BOTTOM FOOTER BAR (Model, Connection Icon, Reset, Configuration)
        egui::TopBottomPanel::bottom("footer_panel")
            .frame(egui::Frame::none().fill(Color32::from_rgb(20, 29, 34)).inner_margin(8.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new("MK870").strong().color(Color32::WHITE));
                    ui.label(RichText::new("⌨").color(COLOR_ACCENT_CYAN));

                    if let Some(msg) = &self.status_message {
                        ui.add_space(16.0);
                        ui.label(RichText::new(format!("✔ {}", msg)).color(COLOR_ACCENT_CYAN).small());
                    } else if let Some(err) = &self.error_message {
                        ui.add_space(16.0);
                        ui.label(RichText::new(format!("⚠ {}", err)).color(Color32::from_rgb(255, 120, 120)).small());
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);
                        let config_name = if self.current_nav == NavTab::FnSetting { "MK870_Fn" } else { "MK870" };
                        ui.label(RichText::new(config_name).color(COLOR_ACCENT_CYAN).underline());
                        ui.label(RichText::new("Configuration:").color(COLOR_TEXT_MUTED));

                        ui.add_space(20.0);
                        if ui.button("↻ Reset").clicked() {
                            self.factory_reset();
                        }
                    });
                });
            });

        // CENTRAL CONTENT AREA
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(COLOR_BG_MAIN).inner_margin(12.0))
            .show(ctx, |ui| {
                match self.current_nav {
                    NavTab::Main | NavTab::FnSetting => {
                        self.show_key_screen(ui);
                    }
                    NavTab::Macro => {
                        render_macro_view(ui, &mut self.macro_state);
                    }
                    NavTab::Light => {
                        self.show_light_screen(ui);
                    }
                    NavTab::Settings => {
                        self.show_settings_screen(ui);
                    }
                    NavTab::About => {
                        self.show_about_screen(ui);
                    }
                }
            });

    }
}

impl KeyboardApp {
    fn show_key_screen(&mut self, ui: &mut egui::Ui) {
        // Redefined indicator line + Fn Layer notification
        ui.horizontal(|ui| {
            let (circle_rect, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
            ui.painter().circle_filled(circle_rect.center(), 4.0, COLOR_ACCENT_CYAN);
            ui.label(RichText::new("The function of highlighted keys has been redefined").color(COLOR_ACCENT_CYAN).small());

            if self.current_nav == NavTab::FnSetting {
                ui.add_space(16.0);
                ui.label(RichText::new("⚡ You are configuring the [Fn + Key] layer. Switch to 'Main' tab to customize standard direct keypresses!").color(Color32::from_rgb(255, 200, 100)).small());
            }
        });

        ui.add_space(6.0);

        // Center Area: Keyboard Canvas on Left + "Current Key Function" Card on Right
        ui.horizontal(|ui| {
            // Keyboard Canvas
            ui.vertical(|ui| {
                let is_fn = self.current_nav == NavTab::FnSetting;
                let redefined = if is_fn { &self.fn_redefined_keys } else { &self.redefined_keys };
                render_keyboard_canvas(
                    ui,
                    KeyboardDisplayMode::KeyMapping,
                    &mut self.selected_key_id,
                    redefined,
                    &mut self.key_colors,
                    self.active_paint_color,
                );
            });

            ui.add_space(16.0);

            // Right Panel: "Current Key Function" Card (matches Screenshot 1)
            ui.vertical(|ui| {
                ui.set_width(230.0);
                egui::Frame::none()
                    .fill(COLOR_BG_PANEL)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .rounding(4.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_min_height(235.0);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new("Current Key Function").strong().color(COLOR_TEXT_ACTIVE));
                            ui.separator();

                            ui.add_space(14.0);
                            let keys = get_tkl_87_layout();
                            if let Some(id) = self.selected_key_id {
                                if let Some(k) = keys.get(id) {
                                    let raw_name = k.name.trim();
                                    let key_name = match raw_name {
                                        "↑" => "Up Arrow",
                                        "↓" => "Down Arrow",
                                        "←" => "Left Arrow",
                                        "→" => "Right Arrow",
                                        "Shift " => "Right Shift",
                                        "Ctrl " => "Right Ctrl",
                                        "Alt " => "Right Alt",
                                        other => other,
                                    };
                                    ui.label(RichText::new(format!("Selected: {}", key_name)).size(16.0).color(COLOR_ACCENT_CYAN).strong());
                                    ui.add_space(4.0);

                                    let is_fn = self.current_nav == NavTab::FnSetting;
                                    let assignment = if is_fn {
                                        self.fn_key_assignments.get(id).cloned().unwrap_or(KeyAssignment::Default)
                                    } else {
                                        self.key_assignments.get(id).cloned().unwrap_or(KeyAssignment::Default)
                                    };

                                    match &assignment {
                                        KeyAssignment::Default => {
                                            ui.label(RichText::new("Status: Default").color(COLOR_TEXT_MUTED));
                                            ui.add_space(10.0);
                                            ui.label(RichText::new("Standard Keystroke").color(COLOR_TEXT_MUTED).small());
                                        }
                                        KeyAssignment::Forbidden => {
                                            ui.label(RichText::new("Status: Customized").color(COLOR_ACCENT_CYAN));
                                            ui.add_space(10.0);
                                            ui.label(RichText::new("Disabled (Forbidden)").color(Color32::from_rgb(255, 120, 120)).small());
                                        }
                                        KeyAssignment::Combination { ctrl, alt, shift, win, key } => {
                                            ui.label(RichText::new("Status: Customized").color(COLOR_ACCENT_CYAN));
                                            ui.add_space(6.0);
                                            let mut parts = Vec::new();
                                            if *ctrl { parts.push("Ctrl"); }
                                            if *alt { parts.push("Alt"); }
                                            if *shift { parts.push("Shift"); }
                                            if *win { parts.push("Win"); }
                                            if !key.is_empty() { parts.push(key.as_str()); }
                                            let combo_str = if parts.is_empty() { "None".to_string() } else { parts.join(" + ") };
                                            ui.label(RichText::new(format!("Combo: {}", combo_str)).color(Color32::WHITE).small());
                                        }
                                        KeyAssignment::Macro { name, mode, loops } => {
                                            ui.label(RichText::new("Status: Customized").color(COLOR_ACCENT_CYAN));
                                            ui.add_space(6.0);
                                            let mode_str = match mode {
                                                MacroTriggerMode::Loop => format!("Loop x{}", loops),
                                                MacroTriggerMode::Auto => "Auto On/Off".to_string(),
                                                MacroTriggerMode::Press => "Press to play".to_string(),
                                            };
                                            ui.label(RichText::new(format!("Macro: {}", name)).color(Color32::WHITE).small());
                                            ui.label(RichText::new(mode_str).color(COLOR_TEXT_MUTED).small());
                                        }
                                        KeyAssignment::Media(m) => {
                                            ui.label(RichText::new("Status: Customized").color(COLOR_ACCENT_CYAN));
                                            ui.add_space(6.0);
                                            ui.label(RichText::new(format!("Media: {}", m)).color(Color32::WHITE).small());
                                        }
                                        KeyAssignment::Mouse(m) => {
                                            ui.label(RichText::new("Status: Customized").color(COLOR_ACCENT_CYAN));
                                            ui.add_space(6.0);
                                            ui.label(RichText::new(format!("Mouse: {}", m)).color(Color32::WHITE).small());
                                        }
                                    }
                                }
                            } else {
                                ui.add_space(25.0);
                                ui.label(RichText::new("No Key Selected").color(COLOR_TEXT_MUTED));
                                ui.add_space(10.0);
                            }

                            ui.add_space(18.0);
                            ui.horizontal(|ui| {
                                let has_key = self.selected_key_id.is_some();

                                if ui.add_enabled(has_key, egui::Button::new("Forbidden")).clicked() {
                                    if let Some(id) = self.selected_key_id {
                                        let is_fn = self.current_nav == NavTab::FnSetting;
                                        if is_fn {
                                            self.fn_redefined_keys[id] = true;
                                            self.fn_key_assignments[id] = KeyAssignment::Forbidden;
                                        } else {
                                            self.redefined_keys[id] = true;
                                            self.key_assignments[id] = KeyAssignment::Forbidden;
                                        }
                                        self.status_message = Some("Key function disabled (Forbidden)".to_string());
                                    }
                                }

                                if ui.add_enabled(has_key, egui::Button::new("Reset")).clicked() {
                                    if let Some(id) = self.selected_key_id {
                                        let is_fn = self.current_nav == NavTab::FnSetting;
                                        if is_fn {
                                            self.fn_redefined_keys[id] = false;
                                            self.fn_key_assignments[id] = KeyAssignment::Default;
                                        } else {
                                            self.redefined_keys[id] = false;
                                            self.key_assignments[id] = KeyAssignment::Default;
                                        }
                                        self.sync_active_macro_bindings();
                                        self.status_message = Some("Key restored to default".to_string());
                                    }
                                }

                                let confirm_btn = egui::Button::new(
                                    RichText::new("Confirm").color(if has_key { COLOR_ACCENT_CYAN } else { COLOR_TEXT_MUTED })
                                );
                                if ui.add_enabled(has_key, confirm_btn).clicked() {
                                    if let Some(id) = self.selected_key_id {
                                        let new_assign = match self.active_assign_type {
                                            AssignType::Combination => {
                                                KeyAssignment::Combination {
                                                    ctrl: self.key_combo_ctrl,
                                                    alt: self.key_combo_alt,
                                                    shift: self.key_combo_shift,
                                                    win: self.key_combo_win,
                                                    key: self.key_combo_char.trim().to_string(),
                                                }
                                            }
                                            AssignType::Macro => {
                                                KeyAssignment::Macro {
                                                    name: self.selected_macro_name.clone(),
                                                    mode: self.macro_trigger_mode,
                                                    loops: self.macro_loop_count,
                                                }
                                            }
                                            AssignType::Media => {
                                                if let Some(ref m) = self.selected_media_action {
                                                    KeyAssignment::Media(m.clone())
                                                } else {
                                                    KeyAssignment::Default
                                                }
                                            }
                                            AssignType::Mouse => {
                                                if let Some(ref m) = self.selected_mouse_action {
                                                    KeyAssignment::Mouse(m.clone())
                                                } else {
                                                    KeyAssignment::Default
                                                }
                                            }
                                        };

                                        let is_fn = self.current_nav == NavTab::FnSetting;
                                        let is_redef = new_assign != KeyAssignment::Default;
                                        if is_fn {
                                            self.fn_redefined_keys[id] = is_redef;
                                            self.fn_key_assignments[id] = new_assign;
                                        } else {
                                            self.redefined_keys[id] = is_redef;
                                            self.key_assignments[id] = new_assign;
                                        }

                                        self.sync_active_macro_bindings();
                                        self.status_message = Some("Key function confirmed! Macro is active.".to_string());
                                    }
                                }
                            });
                        });
                    });
            });
        });

        ui.add_space(10.0);

        // Sub-tabs: Key Setting | Other Setting | Guide & Tutorial
        ui.horizontal(|ui| {
            let (rect1, resp1) = ui.allocate_exact_size(Vec2::new(90.0, 24.0), egui::Sense::click());
            let is_ks = self.key_sub_tab == KeySubTab::KeySetting;
            if resp1.clicked() { self.key_sub_tab = KeySubTab::KeySetting; }
            ui.painter().text(rect1.center(), egui::Align2::CENTER_CENTER, "Key Setting", egui::FontId::proportional(14.0), if is_ks { Color32::WHITE } else { COLOR_TEXT_MUTED });
            if is_ks {
                ui.painter().line_segment([Pos2::new(rect1.left() + 5.0, rect1.bottom()), Pos2::new(rect1.right() - 5.0, rect1.bottom())], Stroke::new(2.0_f32, COLOR_ACCENT_CYAN));
            }

            ui.add_space(8.0);

            let (rect2, resp2) = ui.allocate_exact_size(Vec2::new(90.0, 24.0), egui::Sense::click());
            let is_os = self.key_sub_tab == KeySubTab::OtherSetting;
            if resp2.clicked() { self.key_sub_tab = KeySubTab::OtherSetting; }
            ui.painter().text(rect2.center(), egui::Align2::CENTER_CENTER, "Other Setting", egui::FontId::proportional(14.0), if is_os { Color32::WHITE } else { COLOR_TEXT_MUTED });
            if is_os {
                ui.painter().line_segment([Pos2::new(rect2.left() + 5.0, rect2.bottom()), Pos2::new(rect2.right() - 5.0, rect2.bottom())], Stroke::new(2.0_f32, COLOR_ACCENT_CYAN));
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(12.0);
                let tut_btn = ui.button(RichText::new(if self.show_tutorial { "✖ Close Guide" } else { "❓ Quick Guide & Tutorial" }).color(COLOR_ACCENT_CYAN));
                if tut_btn.clicked() {
                    self.show_tutorial = !self.show_tutorial;
                }
            });
        });

        if self.show_tutorial {
            ui.add_space(4.0);
            egui::Frame::none()
                .fill(Color32::from_rgb(22, 34, 40))
                .stroke(Stroke::new(1.0_f32, COLOR_ACCENT_CYAN))
                .rounding(6.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("📖 Quick Feature Guide & How To Use").strong().color(COLOR_ACCENT_CYAN));
                        ui.add_space(4.0);
                        ui.label(RichText::new("1. Key Setting: Click any key on the keyboard canvas above, choose an action below (Combination, Macro, Media, Mouse), then click 'Confirm'.").color(Color32::WHITE).small());
                        ui.label(RichText::new("2. Combination: Remap a single key to press multiple keys at once (e.g. Ctrl + Shift + Esc for Task Manager, or Ctrl + C for Copy).").color(COLOR_TEXT_ACTIVE).small());
                        ui.label(RichText::new("3. Macro: Automate a sequence of key presses and delays. Set 'Loop' (plays N times), 'auto' (press once to start looping, press again to stop), or 'Press' (loops continuously while holding the key down).").color(COLOR_TEXT_ACTIVE).small());
                        ui.label(RichText::new("4. Media: Turn any key into an audio control (Volume +, Volume -, Mute, Play/Pause, Next Track) or launch Calculator/Browser.").color(COLOR_TEXT_ACTIVE).small());
                        ui.label(RichText::new("5. Mouse: Perform mouse clicks (Left, Right, Middle, Double Click) or scroll the wheel using your keyboard keys.").color(COLOR_TEXT_ACTIVE).small());
                        ui.label(RichText::new("6. FnSetting Tab: Configure secondary shortcuts triggered when holding down the physical Fn key (e.g. Fn + F1 = Volume Up).").color(COLOR_TEXT_MUTED).small());
                    });
                });
            ui.add_space(6.0);
        }

        ui.add_space(6.0);

        if self.key_sub_tab == KeySubTab::KeySetting {
            // Synchronize active selection if selected key changed
            if self.selected_key_id != self.last_selected_key_id {
                self.last_selected_key_id = self.selected_key_id;
                if let Some(id) = self.selected_key_id {
                    let is_fn = self.current_nav == NavTab::FnSetting;
                    let assign = if is_fn {
                        self.fn_key_assignments.get(id)
                    } else {
                        self.key_assignments.get(id)
                    };
                    if let Some(assign) = assign {
                        match assign {
                            KeyAssignment::Combination { ctrl, alt, shift, win, key } => {
                                self.active_assign_type = AssignType::Combination;
                                self.key_combo_ctrl = *ctrl;
                                self.key_combo_alt = *alt;
                                self.key_combo_shift = *shift;
                                self.key_combo_win = *win;
                                self.key_combo_char = key.clone();
                            }
                            KeyAssignment::Macro { name, mode, loops } => {
                                self.active_assign_type = AssignType::Macro;
                                self.selected_macro_name = name.clone();
                                self.macro_trigger_mode = *mode;
                                self.macro_loop_count = *loops;
                            }
                            KeyAssignment::Media(m) => {
                                self.active_assign_type = AssignType::Media;
                                self.selected_media_action = Some(m.clone());
                            }
                            KeyAssignment::Mouse(m) => {
                                self.active_assign_type = AssignType::Mouse;
                                self.selected_mouse_action = Some(m.clone());
                            }
                            _ => {}
                        }
                    }
                }
            }

            // 4 Panels: Combination | Macro | Media | Mouse
            let total_w = ui.available_width();
            let gap = 12.0;
            let card_w = ((total_w - gap * 3.0) / 4.0).floor().max(215.0);
            let card_h = 175.0;

            ui.horizontal(|ui| {
                // 1. Combination
                let is_combo = self.active_assign_type == AssignType::Combination;
                egui::Frame::none()
                    .fill(COLOR_BG_PANEL)
                    .stroke(Stroke::new(1.0_f32, if is_combo { COLOR_ACCENT_CYAN } else { COLOR_BORDER }))
                    .rounding(4.0)
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                    .show(ui, |ui| {
                        ui.set_width(card_w - 20.0);
                        ui.set_height(card_h - 16.0);
                        ui.vertical(|ui| {
                            let title_resp = ui.selectable_label(
                                is_combo,
                                RichText::new("Combination ℹ").strong().color(if is_combo { COLOR_ACCENT_CYAN } else { COLOR_TEXT_ACTIVE })
                            ).on_hover_text("Combination: Bind hotkey shortcuts (like Ctrl + C or Win + D) to a single key press.");
                            if title_resp.clicked() {
                                self.active_assign_type = AssignType::Combination;
                            }
                            ui.add_space(2.0);
                            ui.separator();
                            ui.add_space(6.0);

                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    if ui.checkbox(&mut self.key_combo_ctrl, "Ctrl").clicked() {
                                        self.active_assign_type = AssignType::Combination;
                                    }
                                    if ui.checkbox(&mut self.key_combo_alt, "Alt").clicked() {
                                        self.active_assign_type = AssignType::Combination;
                                    }
                                    if ui.checkbox(&mut self.key_combo_shift, "Shift").clicked() {
                                        self.active_assign_type = AssignType::Combination;
                                    }
                                    if ui.checkbox(&mut self.key_combo_win, "Win").clicked() {
                                        self.active_assign_type = AssignType::Combination;
                                    }
                                });

                                ui.add_space(6.0);

                                ui.vertical(|ui| {
                                    ui.add_space(28.0);
                                    ui.horizontal(|ui| {
                                        ui.label(RichText::new("+").size(15.0).color(COLOR_TEXT_ACTIVE));
                                        let edit = ui.add(
                                            egui::TextEdit::singleline(&mut self.key_combo_char)
                                                .hint_text("Please Enter ⌨")
                                                .desired_width((card_w - 115.0).max(75.0))
                                        );
                                        if edit.changed() || edit.gained_focus() {
                                            self.active_assign_type = AssignType::Combination;
                                        }
                                    });
                                });
                            });
                        });
                    });

                ui.add_space(gap);

                // 2. Macro
                let is_macro = self.active_assign_type == AssignType::Macro;
                egui::Frame::none()
                    .fill(COLOR_BG_PANEL)
                    .stroke(Stroke::new(1.0_f32, if is_macro { COLOR_ACCENT_CYAN } else { COLOR_BORDER }))
                    .rounding(4.0)
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                    .show(ui, |ui| {
                        ui.set_width(card_w - 20.0);
                        ui.set_height(card_h - 16.0);
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                let title_resp = ui.selectable_label(
                                    is_macro,
                                    RichText::new("Macro ℹ").strong().color(if is_macro { COLOR_ACCENT_CYAN } else { COLOR_TEXT_ACTIVE })
                                ).on_hover_text("Macro: Automate multi-key sequences with real-time delays. Press to play, loop, or auto-toggle.");
                                if title_resp.clicked() {
                                    self.active_assign_type = AssignType::Macro;
                                }

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let btn = ui.button(RichText::new("✏").color(COLOR_ACCENT_CYAN));
                                    if btn.clicked() {
                                        self.current_nav = NavTab::Macro;
                                    }
                                    btn.on_hover_text("Open Macro Editor");
                                });
                            });
                            ui.add_space(2.0);
                            ui.separator();
                            ui.add_space(4.0);

                            // Scrollable list of macros
                            ui.allocate_ui_with_layout(
                                Vec2::new(card_w - 20.0, 78.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    egui::ScrollArea::vertical()
                                        .id_source("macro_list_scroll_area")
                                        .max_height(78.0)
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            if self.macro_state.macros.is_empty() {
                                                ui.vertical_centered(|ui| {
                                                    ui.add_space(14.0);
                                                    ui.label(RichText::new("No macros recorded").color(COLOR_TEXT_MUTED).size(12.0));
                                                    if ui.button(RichText::new("Create Macro").size(11.0).color(COLOR_ACCENT_CYAN)).clicked() {
                                                        self.current_nav = NavTab::Macro;
                                                    }
                                                });
                                            } else {
                                                for m in &self.macro_state.macros {
                                                    let is_sel = self.selected_macro_name == m.name;
                                                    let icon = if is_sel { "🔘 " } else { "⚪ " };
                                                    let resp = ui.selectable_label(
                                                        is_sel,
                                                        RichText::new(format!("{}{}", icon, m.name))
                                                            .color(if is_sel { COLOR_ACCENT_CYAN } else { COLOR_TEXT_ACTIVE })
                                                    );
                                                    if resp.clicked() {
                                                        self.selected_macro_name = m.name.clone();
                                                        self.active_assign_type = AssignType::Macro;
                                                    }
                                                }
                                            }
                                        });
                                },
                            );

                            ui.add_space(2.0);
                            ui.separator();
                            ui.add_space(4.0);

                            // Bottom row: Loop [ 1 ]  auto  Press
                            ui.horizontal(|ui| {
                                let mut is_loop = self.macro_trigger_mode == MacroTriggerMode::Loop;
                                if ui.checkbox(&mut is_loop, "Loop").clicked() {
                                    self.macro_trigger_mode = MacroTriggerMode::Loop;
                                    self.active_assign_type = AssignType::Macro;
                                }
                                if self.macro_trigger_mode == MacroTriggerMode::Loop {
                                    ui.add(egui::DragValue::new(&mut self.macro_loop_count).range(1..=99).speed(0.1));
                                }

                                let mut is_auto = self.macro_trigger_mode == MacroTriggerMode::Auto;
                                let auto_cb = ui.checkbox(&mut is_auto, "auto");
                                if auto_cb.clicked() {
                                    self.macro_trigger_mode = MacroTriggerMode::Auto;
                                    self.active_assign_type = AssignType::Macro;
                                }
                                auto_cb.on_hover_text("Button Control On/Off: Press once to loop, press again to stop");

                                let mut is_press = self.macro_trigger_mode == MacroTriggerMode::Press;
                                let press_cb = ui.checkbox(&mut is_press, "Press");
                                if press_cb.clicked() {
                                    self.macro_trigger_mode = MacroTriggerMode::Press;
                                    self.active_assign_type = AssignType::Macro;
                                }
                                press_cb.on_hover_text("Press to play: Loops continuously while held down");
                            });
                        });
                    });

                ui.add_space(gap);

                // 3. Media
                let is_media = self.active_assign_type == AssignType::Media;
                egui::Frame::none()
                    .fill(COLOR_BG_PANEL)
                    .stroke(Stroke::new(1.0_f32, if is_media { COLOR_ACCENT_CYAN } else { COLOR_BORDER }))
                    .rounding(4.0)
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                    .show(ui, |ui| {
                        let card_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(card_w - 20.0, card_h - 16.0));
                        ui.set_clip_rect(card_rect);
                        ui.set_width(card_w - 20.0);
                        ui.set_height(card_h - 16.0);
                        ui.vertical(|ui| {
                            let title_resp = ui.selectable_label(
                                is_media,
                                RichText::new("Media ℹ").strong().color(if is_media { COLOR_ACCENT_CYAN } else { COLOR_TEXT_ACTIVE })
                            ).on_hover_text("Media: Control volume, mute, play/pause music, or launch Calculator/Mail/Browser.");
                            if title_resp.clicked() {
                                self.active_assign_type = AssignType::Media;
                            }
                            ui.add_space(2.0);
                            ui.separator();
                            ui.add_space(4.0);

                            ui.allocate_ui_with_layout(
                                Vec2::new(card_w - 20.0, 122.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    egui::ScrollArea::vertical()
                                        .id_source("media_scroll_area")
                                        .max_height(122.0)
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            let media_ops = [
                                                "Fn", "Player", "Play/Pause", "Stop", "Prev", "Next",
                                                "Volume +", "Volume -", "Mute", "Calculator", "Mail",
                                                "Web Browser", "My Computer", "Search",
                                            ];
                                            for op in media_ops {
                                                let is_sel = self.selected_media_action.as_deref() == Some(op);
                                                let icon = if is_sel { "☑ " } else { "☐ " };
                                                let resp = ui.selectable_label(
                                                    is_sel,
                                                    RichText::new(format!("{}{}", icon, op))
                                                        .color(if is_sel { COLOR_ACCENT_CYAN } else { COLOR_TEXT_ACTIVE })
                                                );
                                                if resp.clicked() {
                                                    if is_sel {
                                                        self.selected_media_action = None;
                                                    } else {
                                                        self.selected_media_action = Some(op.to_string());
                                                        self.active_assign_type = AssignType::Media;
                                                    }
                                                }
                                            }
                                        });
                                },
                            );
                        });
                    });

                ui.add_space(gap);

                // 4. Mouse
                let is_mouse = self.active_assign_type == AssignType::Mouse;
                egui::Frame::none()
                    .fill(COLOR_BG_PANEL)
                    .stroke(Stroke::new(1.0_f32, if is_mouse { COLOR_ACCENT_CYAN } else { COLOR_BORDER }))
                    .rounding(4.0)
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                    .show(ui, |ui| {
                        let card_rect = egui::Rect::from_min_size(ui.cursor().min, Vec2::new(card_w - 20.0, card_h - 16.0));
                        ui.set_clip_rect(card_rect);
                        ui.set_width(card_w - 20.0);
                        ui.set_height(card_h - 16.0);
                        ui.vertical(|ui| {
                            let title_resp = ui.selectable_label(
                                is_mouse,
                                RichText::new("Mouse ℹ").strong().color(if is_mouse { COLOR_ACCENT_CYAN } else { COLOR_TEXT_ACTIVE })
                            ).on_hover_text("Mouse: Emulate mouse clicks (Left, Right, Middle, Double Click) or wheel scrolling.");
                            if title_resp.clicked() {
                                self.active_assign_type = AssignType::Mouse;
                            }
                            ui.add_space(2.0);
                            ui.separator();
                            ui.add_space(4.0);

                            ui.allocate_ui_with_layout(
                                Vec2::new(card_w - 20.0, 122.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    egui::ScrollArea::vertical()
                                        .id_source("mouse_scroll_area")
                                        .max_height(122.0)
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            let mouse_ops = [
                                                "MouseLeft", "MouseRight", "MouseMiddle", "MouseForward",
                                                "MouseBack", "WheelUp", "WheelDown", "DoubleClick",
                                            ];
                                            for op in mouse_ops {
                                                let is_sel = self.selected_mouse_action.as_deref() == Some(op);
                                                let icon = if is_sel { "☑ " } else { "☐ " };
                                                let resp = ui.selectable_label(
                                                    is_sel,
                                                    RichText::new(format!("{}{}", icon, op))
                                                        .color(if is_sel { COLOR_ACCENT_CYAN } else { COLOR_TEXT_ACTIVE })
                                                );
                                                if resp.clicked() {
                                                    if is_sel {
                                                        self.selected_mouse_action = None;
                                                    } else {
                                                        self.selected_mouse_action = Some(op.to_string());
                                                        self.active_assign_type = AssignType::Mouse;
                                                    }
                                                }
                                            }
                                        });
                                },
                            );
                        });
                    });
            });
        } else {
            // Other Setting view
            ui.horizontal(|ui| {
                egui::Frame::none()
                    .fill(COLOR_BG_PANEL)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .rounding(4.0)
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_width(540.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new("Hardware Performance & Wireless Sync").strong().color(COLOR_TEXT_ACTIVE));

                            ui.add_space(10.0);

                            ui.scope(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Polling Rate: ");
                                    let rates = [
                                        (PollingRate::Hz1000, "1000 Hz"),
                                        (PollingRate::Hz500, "500 Hz"),
                                        (PollingRate::Hz250, "250 Hz"),
                                        (PollingRate::Hz125, "125 Hz"),
                                    ];
                                    for (r, label) in rates {
                                        if ui.selectable_label(self.polling_rate == r, label).clicked() {
                                            self.polling_rate = r;
                                            self.apply_polling_rate();
                                        }
                                    }
                                });

                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.label("Switch Debounce: ");
                                    for ms in [2, 4, 8, 16] {
                                        if ui.selectable_label(self.debounce_ms == ms, format!("{} ms", ms)).clicked() {
                                            self.debounce_ms = ms;
                                            self.apply_debounce();
                                        }
                                    }
                                });

                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    ui.label("2.4G Sleep Timeout: ");
                                    if slider_committed(&ui.add(egui::Slider::new(&mut self.sleep_24_min, 1..=30).suffix(" min"))) {
                                        self.apply_sleep();
                                    }
                                });

                                ui.add_space(12.0);
                                ui.horizontal(|ui| {
                                    if ui.button("🕒 Sync Display Clock With PC").clicked() {
                                        self.sync_clock();
                                    }
                                    ui.add_space(12.0);
                                    if ui.button(RichText::new("⚠ Factory Reset").color(Color32::from_rgb(255, 120, 120))).clicked() {
                                        self.factory_reset();
                                    }
                                });
                            });
                        });
                    });
            });
        }
    }



    fn show_light_screen(&mut self, ui: &mut egui::Ui) {
        // Sub-tabs: Light | Light Edit | My Work (matching Screenshot 3)
        ui.horizontal(|ui| {
            let (rect1, resp1) = ui.allocate_exact_size(Vec2::new(70.0, 24.0), egui::Sense::click());
            let is_l = self.light_sub_tab == LightSubTab::Light;
            if resp1.clicked() { self.light_sub_tab = LightSubTab::Light; }
            ui.painter().text(rect1.center(), egui::Align2::CENTER_CENTER, "Light", egui::FontId::proportional(14.0), if is_l { Color32::WHITE } else { COLOR_TEXT_MUTED });
            if is_l {
                ui.painter().line_segment([Pos2::new(rect1.left() + 5.0, rect1.bottom()), Pos2::new(rect1.right() - 5.0, rect1.bottom())], Stroke::new(2.0_f32, COLOR_ACCENT_CYAN));
            }

            ui.add_space(10.0);

            let (rect2, resp2) = ui.allocate_exact_size(Vec2::new(75.0, 24.0), egui::Sense::click());
            let is_le = self.light_sub_tab == LightSubTab::LightEdit;
            if resp2.clicked() { self.light_sub_tab = LightSubTab::LightEdit; }
            ui.painter().text(rect2.center(), egui::Align2::CENTER_CENTER, "Light Edit", egui::FontId::proportional(14.0), if is_le { Color32::WHITE } else { COLOR_TEXT_MUTED });
            if is_le {
                ui.painter().line_segment([Pos2::new(rect2.left() + 5.0, rect2.bottom()), Pos2::new(rect2.right() - 5.0, rect2.bottom())], Stroke::new(2.0_f32, COLOR_ACCENT_CYAN));
            }

            ui.add_space(10.0);

            let (rect3, resp3) = ui.allocate_exact_size(Vec2::new(75.0, 24.0), egui::Sense::click());
            let is_mw = self.light_sub_tab == LightSubTab::MyWork;
            if resp3.clicked() { self.light_sub_tab = LightSubTab::MyWork; }
            ui.painter().text(rect3.center(), egui::Align2::CENTER_CENTER, "My Work", egui::FontId::proportional(14.0), if is_mw { Color32::WHITE } else { COLOR_TEXT_MUTED });
            if is_mw {
                ui.painter().line_segment([Pos2::new(rect3.left() + 5.0, rect3.bottom()), Pos2::new(rect3.right() - 5.0, rect3.bottom())], Stroke::new(2.0_f32, COLOR_ACCENT_CYAN));
            }
        });

        ui.add_space(10.0);

        if self.light_sub_tab == LightSubTab::LightEdit {
            // How Light Edit Works Guidance Banner
            egui::Frame::none()
                .fill(Color32::from_rgb(24, 36, 44))
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .rounding(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🎨 How Per-Key Lighting Works:").strong().color(COLOR_ACCENT_CYAN).small());
                        ui.label(RichText::new("1. Choose a paint color below  ➜  2. Click any key on the canvas to paint it  ➜  3. Click 'Apply Custom Lights to Keyboard' on the right!").color(COLOR_TEXT_ACTIVE).small());
                    });
                });
            ui.add_space(8.0);

            // Per-Key Light Edit Screen
            ui.horizontal(|ui| {
                // Left Area: Illuminated wireframe keyboard + Bottom Tools
                ui.vertical(|ui| {
                    render_keyboard_canvas(
                        ui,
                        KeyboardDisplayMode::LightEdit,
                        &mut self.selected_key_id,
                        &self.redefined_keys,
                        &mut self.key_colors,
                        self.active_paint_color,
                    );

                    ui.add_space(10.0);

                    // Bottom Tools Bar (Color button, Eyedropper, Random Color, Fill, Clear)
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Active Paint Color:").color(COLOR_TEXT_MUTED));
                        
                        // Current color swatch button (clicking toggles color picker popup!)
                        let cur_c = Color32::from_rgb(self.active_paint_color[0], self.active_paint_color[1], self.active_paint_color[2]);
                        let (sw_rect, sw_resp) = ui.allocate_exact_size(Vec2::new(32.0, 24.0), egui::Sense::click());
                        ui.painter().rect_filled(sw_rect, 4.0, cur_c);
                        ui.painter().rect_stroke(sw_rect, 4.0, Stroke::new(1.5_f32, Color32::WHITE));
                        if sw_resp.clicked() {
                            self.color_picker_state.is_open = !self.color_picker_state.is_open;
                            self.color_picker_state.set_rgb(self.active_paint_color[0], self.active_paint_color[1], self.active_paint_color[2]);
                        }
                        sw_resp.on_hover_text("Click to open or close the color palette");

                        if ui.button(RichText::new(if self.color_picker_state.is_open { "✖ Close Picker" } else { "🎨 Pick Color" }).small()).clicked() {
                            self.color_picker_state.is_open = !self.color_picker_state.is_open;
                            if self.color_picker_state.is_open {
                                self.color_picker_state.set_rgb(self.active_paint_color[0], self.active_paint_color[1], self.active_paint_color[2]);
                            }
                        }

                        ui.add_space(8.0);
                        // Fill all keys icon button
                        if ui.button("🪣 Fill All").on_hover_text("Paint all 87 keys with active color").clicked() {
                            for c in &mut self.key_colors {
                                *c = self.active_paint_color;
                            }
                            self.status_message = Some("Painted all keys with active color".to_string());
                        }

                        ui.add_space(4.0);
                        // Clear / Brush icon button
                        if ui.button("🧹 Clear All").on_hover_text("Turn off illumination on all keys (Black)").clicked() {
                            for c in &mut self.key_colors {
                                *c = [0, 0, 0];
                            }
                            self.status_message = Some("Cleared all key illumination".to_string());
                        }
                    });

                    // Color Picker Popup overlay
                    let mut new_color = None;
                    show_color_picker_popup(ui, &mut self.color_picker_state, &mut |c| {
                        new_color = Some(c);
                    });
                    if let Some(c) = new_color {
                        self.active_paint_color = c;
                    }
                });

                ui.add_space(14.0);

                // Right Sidebar: Hardware Illumination & Layer Slots
                ui.vertical(|ui| {
                    ui.set_width(240.0);
                    egui::Frame::none()
                        .fill(COLOR_BG_PANEL)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(6.0)
                        .inner_margin(14.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Hardware Illumination").strong().size(15.0).color(COLOR_TEXT_ACTIVE));
                            ui.label(RichText::new("Upload your custom per-key lighting design to keyboard memory.").small().color(COLOR_TEXT_MUTED));
                            ui.add_space(12.0);

                            // THE PRIMARY SAVE BUTTON
                            let apply_btn = ui.add(
                                egui::Button::new(
                                    RichText::new("💡 Apply Custom Lights to Keyboard")
                                        .strong()
                                        .size(13.0)
                                        .color(Color32::WHITE)
                                )
                                .fill(Color32::from_rgb(0, 160, 140))
                                .min_size(Vec2::new(210.0, 36.0))
                            );
                            if apply_btn.clicked() {
                                // Save current design to active slot
                                match self.light_layer {
                                    1 => self.light_layers.layer_1 = self.key_colors.clone(),
                                    2 => self.light_layers.layer_2 = self.key_colors.clone(),
                                    3 => self.light_layers.layer_3 = self.key_colors.clone(),
                                    _ => {}
                                }
                                crate::storage::save_light_layers(&self.light_layers);
                                self.apply_per_key_rgb();
                            }
                            apply_btn.on_hover_text("Flashes your 87 per-key colors to the keyboard and activates custom illumination!");

                            ui.add_space(14.0);
                            ui.separator();
                            ui.add_space(8.0);

                            // Layer Slot Selector (1, 2, 3)
                            ui.label(RichText::new("Layer Slots:").strong().color(COLOR_TEXT_ACTIVE));
                            ui.horizontal(|ui| {
                                for l in 1..=3 {
                                    let is_sel = self.light_layer == l;
                                    let btn = egui::Button::new(
                                        RichText::new(format!("Slot {}", l))
                                            .color(if is_sel { COLOR_ACCENT_CYAN } else { COLOR_TEXT_MUTED })
                                    );
                                    if ui.add(btn).clicked() {
                                        self.light_layer = l;
                                        let slot_colors = match l {
                                            1 => &self.light_layers.layer_1,
                                            2 => &self.light_layers.layer_2,
                                            3 => &self.light_layers.layer_3,
                                            _ => &self.light_layers.layer_1,
                                        };
                                        if slot_colors.len() == 87 && slot_colors.iter().any(|c| *c != [0, 0, 0]) {
                                            self.key_colors = slot_colors.clone();
                                            self.status_message = Some(format!("Loaded design from Slot {}", l));
                                        }
                                    }
                                }
                            });

                            ui.add_space(8.0);

                            // Explicit Save & Load buttons
                            ui.horizontal(|ui| {
                                if ui.button(RichText::new(format!("💾 Save Slot {}", self.light_layer)).small()).on_hover_text("Save current pattern into this slot").clicked() {
                                    match self.light_layer {
                                        1 => self.light_layers.layer_1 = self.key_colors.clone(),
                                        2 => self.light_layers.layer_2 = self.key_colors.clone(),
                                        3 => self.light_layers.layer_3 = self.key_colors.clone(),
                                        _ => {}
                                    }
                                    crate::storage::save_light_layers(&self.light_layers);
                                    self.status_message = Some(format!("Saved pattern to Slot {}", self.light_layer));
                                }

                                if ui.button(RichText::new(format!("📂 Load Slot {}", self.light_layer)).small()).on_hover_text("Load pattern from this slot into canvas").clicked() {
                                    let slot = match self.light_layer {
                                        1 => &self.light_layers.layer_1,
                                        2 => &self.light_layers.layer_2,
                                        3 => &self.light_layers.layer_3,
                                        _ => &self.light_layers.layer_1,
                                    };
                                    self.key_colors = slot.clone();
                                    self.status_message = Some(format!("Loaded pattern from Slot {}", self.light_layer));
                                }
                            });

                            ui.add_space(14.0);
                            ui.separator();
                            ui.add_space(8.0);

                            // Brightness level
                            ui.label(RichText::new("LED Brightness:").strong().color(COLOR_TEXT_ACTIVE));
                            ui.horizontal(|ui| {
                                for b in 1..=4 {
                                    let is_sel = self.brightness == b;
                                    let btn = egui::Button::new(
                                        RichText::new(format!("{}★", b))
                                            .color(if is_sel { Color32::from_rgb(255, 220, 100) } else { COLOR_TEXT_MUTED })
                                    );
                                    if ui.add(btn).clicked() {
                                        self.brightness = b;
                                    }
                                }
                            });
                        });
                });
            });
        } else {
            // Standard Preset Lighting Studio
            ui.horizontal(|ui| {
                // Left Panel: Lighting Preset Modes & Parameter Controls
                ui.vertical(|ui| {
                    ui.set_width(450.0);
                    egui::Frame::none()
                        .fill(COLOR_BG_PANEL)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(6.0)
                        .inner_margin(14.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Lighting Preset Modes").strong().size(15.0).color(COLOR_TEXT_ACTIVE));
                            ui.add_space(8.0);
                            ui.horizontal_wrapped(|ui| {
                                for mode in LightMode::ALL {
                                    if ui.selectable_label(self.selected_mode == *mode, mode.name()).clicked() {
                                        self.selected_mode = *mode;
                                        self.apply_rgb();
                                    }
                                }
                            });

                            ui.add_space(14.0);
                            ui.separator();
                            ui.add_space(8.0);

                            ui.label(RichText::new("Effect Parameters:").strong().color(COLOR_TEXT_ACTIVE));
                            ui.add_space(4.0);

                            ui.label(format!("Brightness: {} / 4", self.brightness));
                            if slider_committed(&ui.add(egui::Slider::new(&mut self.brightness, 0..=4))) {
                                self.apply_rgb();
                            }

                            ui.add_space(8.0);
                            ui.label(format!("Animation Speed: {} / 4", self.speed));
                            if slider_committed(&ui.add(egui::Slider::new(&mut self.speed, 0..=4))) {
                                self.apply_rgb();
                            }

                            ui.add_space(10.0);
                            if ui.checkbox(&mut self.rainbow, "🌈 Multicolor / Rainbow Dazzle").changed() {
                                self.apply_rgb();
                            }

                            ui.add_space(14.0);
                            ui.horizontal(|ui| {
                                if ui.button(RichText::new("⚡ Apply Preset").color(COLOR_ACCENT_CYAN).strong()).clicked() {
                                    self.apply_rgb();
                                }
                                if ui.button("🌙 Turn Off LEDs").clicked() {
                                    self.selected_mode = LightMode::Off;
                                    self.apply_rgb();
                                }
                            });
                        });
                });

                ui.add_space(14.0);

                // Right Panel: Lighting Color Selection & Palette
                ui.vertical(|ui| {
                    ui.set_width(450.0);
                    egui::Frame::none()
                        .fill(COLOR_BG_PANEL)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(6.0)
                        .inner_margin(14.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Effect Color & Palette").strong().size(15.0).color(COLOR_TEXT_ACTIVE));
                            ui.label(RichText::new("Select the active color for your lighting effect (Static, Breathing, Wave, Reactive, etc.)").small().color(COLOR_TEXT_MUTED));
                            ui.add_space(10.0);

                            // Active Color Preview Card
                            let cur_rgb = self.active_paint_color;
                            let cur_c = Color32::from_rgb(cur_rgb[0], cur_rgb[1], cur_rgb[2]);

                            ui.horizontal(|ui| {
                                // Swatch preview box
                                let (sw_rect, sw_resp) = ui.allocate_exact_size(Vec2::new(48.0, 36.0), egui::Sense::click());
                                ui.painter().rect_filled(sw_rect, 6.0, cur_c);
                                ui.painter().rect_stroke(sw_rect, 6.0, Stroke::new(1.5_f32, Color32::WHITE));
                                if sw_resp.clicked() {
                                    self.color_picker_state.is_open = !self.color_picker_state.is_open;
                                    if self.color_picker_state.is_open {
                                        self.color_picker_state.set_rgb(cur_rgb[0], cur_rgb[1], cur_rgb[2]);
                                    }
                                }
                                sw_resp.on_hover_text("Click to open interactive color picker popup");

                                ui.add_space(8.0);
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(format!("#{:02X}{:02X}{:02X}", cur_rgb[0], cur_rgb[1], cur_rgb[2])).strong().size(14.0).color(COLOR_TEXT_ACTIVE));
                                    ui.label(RichText::new(format!("RGB({}, {}, {})", cur_rgb[0], cur_rgb[1], cur_rgb[2])).small().color(COLOR_TEXT_MUTED));
                                });

                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let pick_btn = ui.add(
                                        egui::Button::new(
                                            RichText::new(if self.color_picker_state.is_open { "✖ Close Picker" } else { "🎨 Custom Color" })
                                                .color(COLOR_ACCENT_CYAN)
                                        )
                                    );
                                    if pick_btn.clicked() {
                                        self.color_picker_state.is_open = !self.color_picker_state.is_open;
                                        if self.color_picker_state.is_open {
                                            self.color_picker_state.set_rgb(cur_rgb[0], cur_rgb[1], cur_rgb[2]);
                                        }
                                    }
                                });
                            });

                            if self.rainbow {
                                ui.add_space(6.0);
                                ui.label(RichText::new("ℹ Rainbow Dazzle mode is currently active. Choosing a color below will switch to single-color mode.").small().color(Color32::from_rgb(255, 200, 100)));
                            }

                            ui.add_space(10.0);
                            ui.separator();
                            ui.add_space(8.0);

                            // Quick Preset Swatches
                            ui.label(RichText::new("Quick Preset Palette:").strong().color(COLOR_TEXT_ACTIVE));
                            ui.add_space(6.0);

                            let presets: &[(&str, [u8; 3])] = &[
                                ("Red", [255, 0, 0]),
                                ("Orange", [255, 120, 0]),
                                ("Amber", [255, 180, 0]),
                                ("Yellow", [255, 255, 0]),
                                ("Lime", [120, 255, 0]),
                                ("Green", [0, 255, 0]),
                                ("Emerald", [0, 255, 128]),
                                ("Cyan", [0, 255, 255]),
                                ("Sky Blue", [0, 160, 255]),
                                ("Blue", [0, 60, 255]),
                                ("Indigo", [80, 0, 200]),
                                ("Purple", [150, 0, 255]),
                                ("Magenta", [255, 0, 200]),
                                ("Pink", [255, 80, 150]),
                                ("Warm White", [255, 220, 180]),
                                ("Pure White", [255, 255, 255]),
                            ];

                            egui::Grid::new("light_preset_color_grid")
                                .spacing([8.0, 8.0])
                                .show(ui, |ui| {
                                    for (i, (name, rgb)) in presets.iter().enumerate() {
                                        let is_active = self.active_paint_color == *rgb && !self.rainbow;
                                        let c = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                                        let (rect, resp) = ui.allocate_exact_size(Vec2::new(44.0, 26.0), egui::Sense::click());
                                        ui.painter().rect_filled(rect, 4.0, c);
                                        let border_color = if is_active {
                                            Color32::WHITE
                                        } else if resp.hovered() {
                                            COLOR_ACCENT_CYAN
                                        } else {
                                            Color32::from_rgb(50, 65, 75)
                                        };
                                        let border_width = if is_active { 2.0_f32 } else { 1.0_f32 };
                                        ui.painter().rect_stroke(rect, 4.0, Stroke::new(border_width, border_color));

                                        if resp.clicked() {
                                            self.active_paint_color = *rgb;
                                            self.rainbow = false;
                                            self.apply_rgb();
                                        }
                                        resp.on_hover_text(format!("{} (#{}:{:02X}{:02X}{:02X})", name, "", rgb[0], rgb[1], rgb[2]));

                                        if (i + 1) % 8 == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });

                            ui.add_space(12.0);
                            ui.separator();
                            ui.add_space(8.0);

                            // Fine-tune RGB Sliders
                            ui.label(RichText::new("RGB Sliders:").strong().color(COLOR_TEXT_ACTIVE));
                            ui.add_space(4.0);

                            let mut r = self.active_paint_color[0];
                            let mut g = self.active_paint_color[1];
                            let mut b = self.active_paint_color[2];
                            let mut slider_commit = false;

                            ui.horizontal(|ui| {
                                ui.label(RichText::new("R").color(Color32::from_rgb(255, 100, 100)).strong());
                                slider_commit |= slider_committed(&ui.add(egui::Slider::new(&mut r, 0..=255)));
                            });
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("G").color(Color32::from_rgb(100, 255, 100)).strong());
                                slider_commit |= slider_committed(&ui.add(egui::Slider::new(&mut g, 0..=255)));
                            });
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("B").color(Color32::from_rgb(100, 150, 255)).strong());
                                slider_commit |= slider_committed(&ui.add(egui::Slider::new(&mut b, 0..=255)));
                            });

                            if [r, g, b] != self.active_paint_color {
                                self.active_paint_color = [r, g, b];
                                self.rainbow = false;
                            }
                            if slider_commit {
                                self.apply_rgb();
                            }

                            // Interactive Popup overlay if opened
                            let mut popup_new_color = None;
                            show_color_picker_popup(ui, &mut self.color_picker_state, &mut |c| {
                                popup_new_color = Some(c);
                            });
                            if let Some(c) = popup_new_color {
                                self.active_paint_color = c;
                                self.rainbow = false;
                                self.apply_rgb();
                            }
                        });
                });
            });
        }
    }

    fn show_about_screen(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            // Left Card: Device Info & Privacy Guarantee
            ui.vertical(|ui| {
                ui.set_width(470.0);
                egui::Frame::none()
                    .fill(COLOR_BG_PANEL)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .rounding(6.0)
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Device Information").strong().size(16.0).color(COLOR_TEXT_ACTIVE));
                        ui.add_space(8.0);
                        if let Some(info) = &self.info {
                            ui.label(format!("Device Model:   {}", info.name));
                            ui.label(format!("Hardware VID:   0x{:04X}", info.vid));
                            ui.label(format!("Hardware PID:   0x{:04X}", info.pid));
                            ui.label(format!("Connection:     {}", if info.is_wireless { "2.4 GHz Wireless Dongle" } else { "USB Cable (Wired Mode)" }));
                            if let Some(bat) = info.battery_percent {
                                ui.label(format!("Battery Level:  {}%", bat));
                            }
                            ui.label(format!("Firmware Date:  {}", info.firmware_date));
                        } else {
                            ui.label("Device is disconnected. Connect via 2.4G dongle or USB cable.");
                        }

                        ui.add_space(14.0);
                        ui.separator();
                        ui.add_space(10.0);

                        ui.label(RichText::new("🛡 100% Offline & Clean Guarantee").strong().color(COLOR_ACCENT_CYAN));
                        ui.add_space(6.0);
                        ui.label("This driver was reverse-engineered from the vendor package to guarantee complete privacy and eliminate bloatware:");
                        ui.add_space(4.0);
                        ui.label("❌ Removed: Google Analytics telemetry");
                        ui.label("❌ Removed: Hardware fingerprinting tracker");
                        ui.label("❌ Removed: Chinese cloud phone-home");
                        ui.label("❌ Removed: Mandatory cloud logins & accounts");
                        ui.label("❌ Removed: 200MB Electron/Chromium runtime");
                        ui.add_space(6.0);
                        ui.label("✔ 100% Offline: zero network sockets, zero outbound telemetry");
                        ui.label("✔ Direct Native Win32 HID: instant response with 25MB RAM");

                        ui.add_space(14.0);
                        ui.separator();
                        ui.add_space(10.0);

                        ui.label(RichText::new("Hardware Modes:").strong().color(Color32::WHITE));
                        ui.add_space(4.0);
                        ui.label(RichText::new("2.4G and USB modes both support lighting, polling rate, debounce, and sleep settings.\nOver 2.4G the keyboard must be awake: press any key if it has gone to sleep.").small().color(COLOR_TEXT_MUTED));
                    });
            });

            ui.add_space(16.0);

            // Right Card: Comprehensive Feature Guide & Tutorial
            ui.vertical(|ui| {
                ui.set_width(490.0);
                egui::Frame::none()
                    .fill(COLOR_BG_PANEL)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .rounding(6.0)
                    .inner_margin(16.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new("📖 Feature Guide & Tutorial").strong().size(16.0).color(COLOR_ACCENT_CYAN));
                        ui.add_space(8.0);

                        egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
                            ui.label(RichText::new("1. Key Setting").strong().color(Color32::WHITE));
                            ui.label("Allows you to reprogram any physical key on your MK870. Simply click a key on the visual keyboard canvas, choose one of the 4 functions below, and click Confirm.");
                            ui.add_space(8.0);

                            ui.label(RichText::new("2. Combination (Hotkeys)").strong().color(Color32::WHITE));
                            ui.label("Binds modifier combinations to a single key press. For example, check 'Ctrl' and enter 'C' to make that key perform Copy, or check 'Win' and enter 'D' to show the desktop.");
                            ui.add_space(8.0);

                            ui.label(RichText::new("3. Macro (Automation)").strong().color(Color32::WHITE));
                            ui.label("Executes a recorded sequence of keystrokes with millisecond-accurate timing. You can choose from 3 playback modes:");
                            ui.label(" • Loop [N]: Plays the macro N times when pressed.");
                            ui.label(" • auto: Toggles continuous looping on/off with each key press.");
                            ui.label(" • Press: Loops repeatedly while holding the key down, and stops immediately when released.");
                            ui.add_space(8.0);

                            ui.label(RichText::new("4. Media (Multimedia Controls)").strong().color(Color32::WHITE));
                            ui.label("Turns any key into a multimedia shortcut: Volume +, Volume -, Mute, Play/Pause, Next Track, Calculator, Mail, or Web Browser.");
                            ui.add_space(8.0);

                            ui.label(RichText::new("5. Mouse (Emulation)").strong().color(Color32::WHITE));
                            ui.label("Allows triggering mouse actions from your keyboard: Left Click, Right Click, Middle Click, Double Click, or Wheel Up/Down.");
                            ui.add_space(8.0);

                            ui.label(RichText::new("6. FnSetting Layer").strong().color(Color32::WHITE));
                            ui.label("Configures the secondary layer of your keyboard. Keys mapped in FnSetting are triggered only when holding down the physical Fn key (e.g. Fn + F1).");
                        });
                    });
            });
        });
    }

    fn save_app_settings(&self) {
        let s = crate::storage::AppSettings {
            ui_scale: self.ui_scale,
            font_size_delta: self.font_size_delta,
        };
        crate::storage::save_settings(&s);
    }

    fn show_settings_screen(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(4.0);

            // Header Banner
            egui::Frame::none()
                .fill(COLOR_BG_PANEL)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .rounding(6.0)
                .inner_margin(12.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("⚙ Application & Accessibility Settings").strong().size(18.0).color(COLOR_TEXT_ACTIVE));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if let Some(msg) = &self.status_message {
                                ui.label(RichText::new(format!("ℹ {}", msg)).color(COLOR_ACCENT_CYAN).small());
                            }
                        });
                    });
                    ui.add_space(4.0);
                    ui.label(RichText::new("Configure text readability, UI scaling, and easily backup or import your custom macros.")
                        .color(COLOR_TEXT_MUTED).size(13.0));
                });

            ui.add_space(12.0);

            // Section 1: Display Scaling & Text Size
            egui::Frame::none()
                .fill(COLOR_BG_PANEL)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .rounding(6.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("🔍 Display Scaling & Text Size").strong().size(16.0).color(COLOR_ACCENT_CYAN));
                    ui.label(RichText::new("Scale up the interface and text to make everything easy to read on 1080p, 1440p, or 4K monitors.").color(COLOR_TEXT_MUTED).small());
                    ui.add_space(12.0);

                    // Quick Presets
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Size Presets:").color(COLOR_TEXT_ACTIVE).strong());
                        ui.add_space(8.0);
                        let presets = [
                            ("100% Normal", 1.00_f32, 0.0_f32),
                            ("115% Comfortable", 1.15_f32, 0.0_f32),
                            ("125% Large", 1.25_f32, 1.0_f32),
                            ("140% Extra Large", 1.40_f32, 2.0_f32),
                            ("160% Huge (4K)", 1.60_f32, 3.0_f32),
                        ];
                        for (label, scale, font_delta) in presets {
                            let is_active = (self.ui_scale - scale).abs() < 0.02 && (self.font_size_delta - font_delta).abs() < 0.5;
                            let btn = egui::Button::new(
                                RichText::new(label).color(if is_active { COLOR_ACCENT_CYAN } else { COLOR_TEXT_ACTIVE })
                            );
                            if ui.add(btn).clicked() {
                                self.ui_scale = scale;
                                self.font_size_delta = font_delta;
                                self.save_app_settings();
                            }
                        }
                    });

                    ui.add_space(12.0);

                    // Granular Zoom Slider
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Fine Zoom:").color(COLOR_TEXT_ACTIVE));
                        ui.add_space(8.0);

                        if ui.button("➖ 5%").on_hover_text("Decrease scale by 5%").clicked() {
                            self.ui_scale = ((self.ui_scale - 0.05) * 100.0).round() / 100.0;
                            self.ui_scale = self.ui_scale.max(0.80);
                            self.save_app_settings();
                        }

                        let mut pct = (self.ui_scale * 100.0).round() as u32;
                        if ui.add(egui::Slider::new(&mut pct, 80..=200).suffix("%").text("UI Scale")).changed() {
                            self.ui_scale = pct as f32 / 100.0;
                            self.save_app_settings();
                        }

                        if ui.button("➕ 5%").on_hover_text("Increase scale by 5%").clicked() {
                            self.ui_scale = ((self.ui_scale + 0.05) * 100.0).round() / 100.0;
                            self.ui_scale = self.ui_scale.min(2.00);
                            self.save_app_settings();
                        }

                        ui.add_space(8.0);
                        if ui.button("↺ Reset (100%)").clicked() {
                            self.ui_scale = 1.00;
                            self.font_size_delta = 0.0;
                            self.save_app_settings();
                        }
                    });

                    ui.add_space(10.0);

                    // Font Size Boost
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Font Boost:").color(COLOR_TEXT_ACTIVE));
                        ui.add_space(8.0);
                        let font_opts = [
                            ("Normal (+0pt)", 0.0_f32),
                            ("Medium (+2pt)", 2.0_f32),
                            ("Large (+4pt)", 4.0_f32),
                        ];
                        for (name, delta) in font_opts {
                            let is_active = (self.font_size_delta - delta).abs() < 0.1;
                            let btn = egui::Button::new(
                                RichText::new(name).color(if is_active { COLOR_ACCENT_CYAN } else { COLOR_TEXT_ACTIVE })
                            );
                            if ui.add(btn).clicked() {
                                self.font_size_delta = delta;
                                self.save_app_settings();
                            }
                        }
                    });

                    ui.add_space(14.0);

                    // Live Typography Preview
                    egui::Frame::none()
                        .fill(COLOR_BG_CANVAS)
                        .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                        .rounding(4.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Live Readability Preview:").small().color(COLOR_TEXT_MUTED));
                            ui.add_space(4.0);
                            ui.label(RichText::new("Heading Sample: FL-ESPORTS MK870 Driver").heading().color(COLOR_TEXT_ACTIVE));
                            ui.label(RichText::new("Body Sample: This text scales live with your settings. Key remapping, RGB effects, and macros run 100% offline with zero cloud overhead.").color(COLOR_TEXT_ACTIVE));
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                let _ = ui.button("Sample Button");
                                ui.label(RichText::new("Keycode: [ 0x05AC / 0x024F ]").monospace().color(COLOR_ACCENT_CYAN));
                            });
                        });
                });

            ui.add_space(12.0);

            // Section 2: Macro Backup, Import & Export
            egui::Frame::none()
                .fill(COLOR_BG_PANEL)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .rounding(6.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("📦 Macro Backup, Import & Export").strong().size(16.0).color(COLOR_ACCENT_CYAN));
                    ui.label(RichText::new("Export your macros to standard human-readable JSON files for backup or sharing, or import macro packs.").color(COLOR_TEXT_MUTED).small());
                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("Currently loaded macros: {} in library", self.macro_state.macros.len())).color(COLOR_TEXT_ACTIVE).strong());
                    });
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("📤 Export All Macros (.json)").color(COLOR_ACCENT_CYAN)).on_hover_text("Open Windows dialog to save all macros to a JSON file").clicked() {
                            if let Some(msg) = crate::storage::export_macros_to_file(&self.macro_state.macros) {
                                self.status_message = Some(msg);
                            }
                        }

                        if ui.button(RichText::new("📥 Import Macros (.json)").color(Color32::from_rgb(100, 220, 255))).on_hover_text("Open Windows dialog to load macros from a JSON file").clicked() {
                            match crate::storage::import_macros_from_file() {
                                Ok((imported_macros, msg)) => {
                                    for item in imported_macros {
                                        if let Some(existing) = self.macro_state.macros.iter_mut().find(|m| m.name == item.name) {
                                            *existing = item;
                                        } else {
                                            self.macro_state.macros.push(item);
                                        }
                                    }
                                    crate::storage::save_macros(&self.macro_state.macros);
                                    self.sync_active_macro_bindings();
                                    self.status_message = Some(msg);
                                }
                                Err(e) if e != "Cancelled" => {
                                    self.status_message = Some(format!("Import failed: {}", e));
                                }
                                _ => {}
                            }
                        }

                        if ui.button("📁 Open Config Folder").on_hover_text("Open local directory where settings.json and macros.json are stored").clicked() {
                            let dir = crate::storage::get_storage_dir();
                            let _ = std::process::Command::new("explorer").arg(&dir).spawn();
                        }

                        if ui.button("↺ Reset to Example Macros").on_hover_text("Restore default factory macro examples").clicked() {
                            self.macro_state = MacroViewState::default();
                            crate::storage::save_macros(&self.macro_state.macros);
                            self.sync_active_macro_bindings();
                            self.status_message = Some("Restored default factory macros".to_string());
                        }
                    });
                });

            ui.add_space(12.0);

            // Section 3: Privacy & Storage Info
            egui::Frame::none()
                .fill(COLOR_BG_PANEL)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .rounding(6.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("🛡 Privacy, Storage & Device").strong().size(16.0).color(COLOR_TEXT_ACTIVE));
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let (circ, _) = ui.allocate_exact_size(Vec2::new(10.0, 10.0), egui::Sense::hover());
                        ui.painter().circle_filled(circ.center(), 4.0, Color32::from_rgb(0, 255, 150));
                        ui.label(RichText::new("100% Offline").color(Color32::from_rgb(0, 255, 150)).strong());
                        ui.label(RichText::new("• All settings and macros are saved locally. No analytics, no telemetry, no internet required.").color(COLOR_TEXT_MUTED).small());
                    });
                });
        });
    }

}
