use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

use eframe::egui;
use egui::{Color32, RichText, Stroke};
use super::theme::*;

#[link(name = "user32")]
extern "system" {
    fn GetAsyncKeyState(vKey: i32) -> i16;
    fn GetKeyNameTextW(lParam: i32, lpString: *mut u16, cchSize: i32) -> i32;
    fn MapVirtualKeyW(uCode: u32, uMapType: u32) -> u32;
    fn keybd_event(bVk: u8, bScan: u8, dwFlags: u32, dwExtraInfo: usize);
    fn VkKeyScanW(ch: u16) -> i16;
}

pub fn name_to_vk(name: &str) -> Option<u8> {
    let clean = name.trim();
    match clean.to_uppercase().as_str() {
        "SPACE" => Some(0x20),
        "ENTER" | "RETURN" => Some(0x0D),
        "TAB" => Some(0x09),
        "BACKSPACE" => Some(0x08),
        "ESC" | "ESCAPE" => Some(0x1B),
        "SHIFT" => Some(0x10),
        "CTRL" | "CONTROL" => Some(0x11),
        "ALT" => Some(0x12),
        "CAPS LOCK" | "CAPSLOCK" => Some(0x14),
        "UP" | "UP ARROW" | "↑" => Some(0x26),
        "DOWN" | "DOWN ARROW" | "↓" => Some(0x28),
        "LEFT" | "LEFT ARROW" | "←" => Some(0x25),
        "RIGHT" | "RIGHT ARROW" | "→" => Some(0x27),
        "INSERT" | "INS" => Some(0x2D),
        "DELETE" | "DEL" => Some(0x2E),
        "HOME" => Some(0x24),
        "END" => Some(0x23),
        "PAGE UP" | "PGUP" => Some(0x21),
        "PAGE DOWN" | "PGDN" => Some(0x22),
        s if s.starts_with('F') && s.len() <= 3 => {
            if let Ok(num) = s[1..].parse::<u8>() {
                if (1..=24).contains(&num) {
                    Some(0x70 + num - 1)
                } else {
                    None
                }
            } else {
                None
            }
        }
        s if s.len() == 1 => {
            let ch = s.chars().next().unwrap();
            let res = unsafe { VkKeyScanW(ch as u16) };
            if res != -1 {
                Some((res & 0xFF) as u8)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn play_macro_sequence(actions: &[MacroAction]) {
    for act in actions {
        match act {
            MacroAction::KeyDown(k) => {
                if let Some(vk) = name_to_vk(k) {
                    let scan = unsafe { MapVirtualKeyW(vk as u32, 0) } as u8;
                    let ext = if (33..=46).contains(&vk) || (91..=93).contains(&vk) { 0x0001 } else { 0 };
                    unsafe { keybd_event(vk, scan, ext, 0) };
                }
            }
            MacroAction::KeyUp(k) => {
                if let Some(vk) = name_to_vk(k) {
                    let scan = unsafe { MapVirtualKeyW(vk as u32, 0) } as u8;
                    let ext = if (33..=46).contains(&vk) || (91..=93).contains(&vk) { 0x0001 } else { 0 };
                    unsafe { keybd_event(vk, scan, 0x0002 | ext, 0) };
                }
            }
            MacroAction::Delay(ms) => {
                std::thread::sleep(Duration::from_millis((*ms as u64).min(5000)));
            }
        }
    }
}

fn vk_to_name(vk: u32) -> String {
    let scan = unsafe { MapVirtualKeyW(vk, 0) };
    let ext = if (33..=46).contains(&vk) || (91..=93).contains(&vk) || vk == 0x2D || vk == 0x2E {
        1 << 24
    } else {
        0
    };
    let l_param = ((scan << 16) | ext) as i32;
    let mut buf = [0u16; 64];
    let len = unsafe { GetKeyNameTextW(l_param, buf.as_mut_ptr(), 64) };
    if len > 0 {
        String::from_utf16_lossy(&buf[..len as usize])
    } else {
        match vk {
            0x08 => "Backspace".to_string(),
            0x09 => "Tab".to_string(),
            0x0D => "Enter".to_string(),
            0x10 => "Shift".to_string(),
            0x11 => "Ctrl".to_string(),
            0x12 => "Alt".to_string(),
            0x14 => "Caps Lock".to_string(),
            0x1B => "Esc".to_string(),
            0x20 => "Space".to_string(),
            0x5B => "Win".to_string(),
            0x5C => "Right Win".to_string(),
            0x60..=0x69 => format!("Num {}", vk - 0x60),
            0x6A => "Num *".to_string(),
            0x6B => "Num +".to_string(),
            0x6D => "Num -".to_string(),
            0x6E => "Num .".to_string(),
            0x6F => "Num /".to_string(),
            0x70..=0x87 => format!("F{}", vk - 0x70 + 1),
            0x30..=0x39 => ((vk as u8) as char).to_string(),
            0x41..=0x5A => ((vk as u8) as char).to_string(),
            _ => format!("Key_{}", vk),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum MacroAction {
    KeyDown(String),
    KeyUp(String),
    Delay(u32),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct MacroItem {
    pub name: String,
    pub loop_count: u32,
    pub button_control: bool,
    pub press_to_play: bool,
    pub fixed_delay_enabled: bool,
    pub fixed_delay_ms: u32,
    pub actions: Vec<MacroAction>,
}

impl MacroItem {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            loop_count: 1,
            button_control: false,
            press_to_play: false,
            fixed_delay_enabled: false,
            fixed_delay_ms: 20,
            actions: Vec::new(),
        }
    }
}

pub struct MacroRecorder {
    running: Arc<AtomicBool>,
    rx: Receiver<(String, bool, Instant)>,
}

impl MacroRecorder {
    pub fn start() -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let (tx, rx) = channel();

        let r_clone = Arc::clone(&running);
        std::thread::spawn(move || {
            let mut states = [false; 256];
            // Initialize current states so already-held keys aren't treated as new presses
            for vk in 1..256 {
                states[vk] = unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 };
            }

            while r_clone.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(4));
                let now = Instant::now();

                for vk in 1..256 {
                    // Skip mouse buttons (Left=1, Right=2, Middle=4, X1=5, X2=6)
                    if vk == 1 || vk == 2 || vk == 4 || vk == 5 || vk == 6 {
                        continue;
                    }

                    let is_down = unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 };

                    if is_down && !states[vk] {
                        states[vk] = true;
                        let name = vk_to_name(vk as u32);
                        let _ = tx.send((name, true, now));
                    } else if !is_down && states[vk] {
                        states[vk] = false;
                        let name = vk_to_name(vk as u32);
                        let _ = tx.send((name, false, now));
                    }
                }
            }
        });

        Self { running, rx }
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }

    pub fn try_recv(&self) -> Option<(String, bool, Instant)> {
        self.rx.try_recv().ok()
    }
}

impl Drop for MacroRecorder {
    fn drop(&mut self) {
        self.stop();
    }
}

pub struct MacroViewState {
    pub macros: Vec<MacroItem>,
    pub selected_idx: usize,
    pub is_recording: bool,
    pub status_msg: Option<String>,
    pub recorder: Option<MacroRecorder>,
    pub last_instant: Option<Instant>,
    pub last_shift: bool,
    pub last_ctrl: bool,
    pub last_alt: bool,
}

impl Default for MacroViewState {
    fn default() -> Self {
        if let Some(saved) = crate::storage::load_saved_macros() {
            if !saved.is_empty() {
                return Self {
                    macros: saved,
                    selected_idx: 0,
                    is_recording: false,
                    status_msg: None,
                    recorder: None,
                    last_instant: None,
                    last_shift: false,
                    last_ctrl: false,
                    last_alt: false,
                };
            }
        }

        let mut m1 = MacroItem::new("Macro 1");
        m1.actions = vec![
            MacroAction::KeyDown("W".to_string()),
            MacroAction::Delay(45),
            MacroAction::KeyUp("W".to_string()),
            MacroAction::Delay(60),
            MacroAction::KeyDown("SPACE".to_string()),
            MacroAction::Delay(30),
            MacroAction::KeyUp("SPACE".to_string()),
        ];

        let mut m2 = MacroItem::new("Quick Combo");
        m2.actions = vec![
            MacroAction::KeyDown("Q".to_string()),
            MacroAction::Delay(50),
            MacroAction::KeyUp("Q".to_string()),
            MacroAction::Delay(80),
            MacroAction::KeyDown("E".to_string()),
            MacroAction::Delay(50),
            MacroAction::KeyUp("E".to_string()),
        ];

        Self {
            macros: vec![m1, m2, MacroItem::new("Macro 3")],
            selected_idx: 0,
            is_recording: false,
            status_msg: None,
            recorder: None,
            last_instant: None,
            last_shift: false,
            last_ctrl: false,
            last_alt: false,
        }
    }
}

pub fn render_macro_view(ui: &mut egui::Ui, state: &mut MacroViewState) {
    // Process live recording
    if state.is_recording {
        ui.ctx().request_repaint();

        let mut incoming = Vec::new();

        // Source 1: Background hardware polling (captures global & background keys)
        if let Some(recorder) = &state.recorder {
            while let Some(evt) = recorder.try_recv() {
                incoming.push(evt);
            }
        }

        // Source 2: Egui native input events (captures focused window keys)
        ui.input(|i| {
            // Track modifier state transitions
            if i.modifiers.shift != state.last_shift {
                state.last_shift = i.modifiers.shift;
                incoming.push(("Shift".to_string(), state.last_shift, Instant::now()));
            }
            if i.modifiers.ctrl != state.last_ctrl {
                state.last_ctrl = i.modifiers.ctrl;
                incoming.push(("Ctrl".to_string(), state.last_ctrl, Instant::now()));
            }
            if i.modifiers.alt != state.last_alt {
                state.last_alt = i.modifiers.alt;
                incoming.push(("Alt".to_string(), state.last_alt, Instant::now()));
            }

            for event in &i.events {
                if let egui::Event::Key { key, pressed, repeat, .. } = event {
                    if !*repeat {
                        let kname = key.name().to_string();
                        // Deduplicate if already reported by hardware poll in this frame
                        if !incoming.iter().any(|(n, down, _)| n.eq_ignore_ascii_case(&kname) && *down == *pressed) {
                            incoming.push((kname, *pressed, Instant::now()));
                        }
                    }
                }
            }
        });

        if let Some(macro_item) = state.macros.get_mut(state.selected_idx) {
            for (key_name, is_down, now) in incoming {
                if let Some(last_time) = state.last_instant {
                    let elapsed = now.duration_since(last_time).as_millis() as u32;
                    if elapsed > 0 && !macro_item.actions.is_empty() {
                        let delay = if macro_item.fixed_delay_enabled {
                            macro_item.fixed_delay_ms
                        } else {
                            elapsed.clamp(1, 5000)
                        };
                        macro_item.actions.push(MacroAction::Delay(delay));
                    }
                }
                state.last_instant = Some(now);

                if is_down {
                    macro_item.actions.push(MacroAction::KeyDown(key_name));
                } else {
                    macro_item.actions.push(MacroAction::KeyUp(key_name));
                }
            }
        }
    }

    ui.horizontal(|ui| {
        // Left Column: Macro Tree / List
        ui.vertical(|ui| {
            ui.set_width(220.0);
            
            // Header: "Hot" + icons
            egui::Frame::none()
                .fill(COLOR_BG_PANEL)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Hot").strong().color(COLOR_TEXT_ACTIVE));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("🗑").on_hover_text("Delete Macro").clicked() {
                                if state.macros.len() > 1 {
                                    state.macros.remove(state.selected_idx);
                                    if state.selected_idx >= state.macros.len() {
                                        state.selected_idx = state.macros.len().saturating_sub(1);
                                    }
                                    crate::storage::save_macros(&state.macros);
                                }
                            }
                            if ui.button("📁").on_hover_text("New Folder").clicked() {
                                state.status_msg = Some("Created folder: Hot/Group".to_string());
                            }
                            if ui.button("+").on_hover_text("Add Macro").clicked() {
                                let new_name = format!("Macro {}", state.macros.len() + 1);
                                state.macros.push(MacroItem::new(&new_name));
                                state.selected_idx = state.macros.len() - 1;
                                crate::storage::save_macros(&state.macros);
                            }
                        });
                    });
                });

            ui.add_space(4.0);

            // Macro list body
            egui::Frame::none()
                .fill(COLOR_BG_CANVAS)
                .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.set_min_height(420.0);
                    for (i, m) in state.macros.iter().enumerate() {
                        let is_sel = state.selected_idx == i;
                        let item_text = RichText::new(&m.name)
                            .color(if is_sel { COLOR_ACCENT_CYAN } else { COLOR_TEXT_ACTIVE });
                        
                        let resp = ui.selectable_label(is_sel, item_text);
                        if resp.clicked() {
                            state.selected_idx = i;
                        }
                    }
                });

            ui.add_space(4.0);

            // Import & Export buttons
            ui.horizontal(|ui| {
                if ui.button(RichText::new("📤 Export").small()).on_hover_text("Export all macros to a JSON file").clicked() {
                    if let Some(msg) = crate::storage::export_macros_to_file(&state.macros) {
                        state.status_msg = Some(msg);
                    }
                }
                if ui.button(RichText::new("📥 Import").small()).on_hover_text("Import macros from a JSON file").clicked() {
                    match crate::storage::import_macros_from_file() {
                        Ok((items, msg)) => {
                            for item in items {
                                if let Some(existing) = state.macros.iter_mut().find(|m| m.name == item.name) {
                                    *existing = item;
                                } else {
                                    state.macros.push(item);
                                }
                            }
                            crate::storage::save_macros(&state.macros);
                            state.status_msg = Some(msg);
                        }
                        Err(e) if e != "Cancelled" => {
                            state.status_msg = Some(format!("Error: {}", e));
                        }
                        _ => {}
                    }
                }
            });
        });

        ui.add_space(12.0);

        // Right Column: Macro Editor
        ui.vertical(|ui| {
            let mut should_save_macros = false;
            if let Some(macro_item) = state.macros.get_mut(state.selected_idx) {
                // Top controls row (Loop, Button Control, Press to play)
                egui::Frame::none()
                    .fill(COLOR_BG_PANEL)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mut has_loop = macro_item.loop_count > 0;
                            if ui.checkbox(&mut has_loop, "Loop").changed() {
                                macro_item.loop_count = if has_loop { 1 } else { 0 };
                            }
                            ui.add(egui::DragValue::new(&mut macro_item.loop_count).range(1..=999).speed(1.0));

                            ui.add_space(24.0);
                            ui.checkbox(&mut macro_item.button_control, "Button Control On/Off");

                            ui.add_space(24.0);
                            ui.checkbox(&mut macro_item.press_to_play, "Press to play");
                        });
                    });

                ui.add_space(6.0);

                // Action Bar (Start/Stop, Add, Clear, Fix [X] ms, Save, (?))
                egui::Frame::none()
                    .fill(Color32::from_rgb(28, 42, 49))
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let rec_btn_text = if state.is_recording {
                                RichText::new("⏹ Stop").color(Color32::from_rgb(255, 90, 90)).strong()
                            } else {
                                RichText::new("⏺ Start").color(COLOR_ACCENT_CYAN).strong()
                            };

                            if ui.button(rec_btn_text).clicked() {
                                if state.is_recording {
                                    // Stop recording
                                    if let Some(rec) = state.recorder.take() {
                                        rec.stop();
                                    }
                                    state.is_recording = false;
                                    state.last_instant = None;
                                    state.status_msg = Some(format!("Recording complete! Captured {} actions.", macro_item.actions.len()));
                                } else {
                                    // Start recording
                                    state.recorder = Some(MacroRecorder::start());
                                    state.is_recording = true;
                                    state.last_instant = None;
                                    state.last_shift = false;
                                    state.last_ctrl = false;
                                    state.last_alt = false;
                                    state.status_msg = Some("⏺ Recording active! Type keys on your keyboard... Click Stop when finished.".to_string());
                                }
                            }

                            if ui.button("Add").clicked() {
                                macro_item.actions.push(MacroAction::KeyDown("SPACE".to_string()));
                                macro_item.actions.push(MacroAction::Delay(20));
                                macro_item.actions.push(MacroAction::KeyUp("SPACE".to_string()));
                            }

                            if ui.button("Clear").clicked() {
                                macro_item.actions.clear();
                            }

                            let test_btn = ui.button(RichText::new("▶ Test Run").color(COLOR_ACCENT_CYAN));
                            if test_btn.clicked() {
                                let acts = macro_item.actions.clone();
                                state.status_msg = Some("▶ Executing macro in 1 second... (switch to target window)".to_string());
                                std::thread::spawn(move || {
                                    std::thread::sleep(Duration::from_millis(1000));
                                    play_macro_sequence(&acts);
                                });
                            }
                            test_btn.on_hover_text("Executes this macro after a 1-second delay so you can switch to any text editor or game to test it.");

                            ui.add_space(16.0);
                            ui.checkbox(&mut macro_item.fixed_delay_enabled, "Fix");
                            ui.add(egui::DragValue::new(&mut macro_item.fixed_delay_ms).range(1..=5000).speed(1.0));
                            ui.label("ms");

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new("(?)").color(COLOR_TEXT_MUTED)).on_hover_text("Records real-time keystrokes and delays. Click Start, type your keys, then click Stop.");
                                if ui.add(egui::Button::new(RichText::new("Save").strong().color(COLOR_ACCENT_CYAN))).clicked() {
                                    should_save_macros = true;
                                    state.status_msg = Some(format!("Macro '{}' saved successfully!", macro_item.name));
                                }
                                if ui.button(RichText::new("📤 Export This").small()).on_hover_text("Export this specific macro to a JSON file").clicked() {
                                    if let Some(msg) = crate::storage::export_single_macro_to_file(macro_item) {
                                        state.status_msg = Some(msg);
                                    }
                                }
                            });
                        });
                    });

                // Recording status indicator banner
                if state.is_recording {
                    ui.add_space(4.0);
                    egui::Frame::none()
                        .fill(Color32::from_rgb(45, 25, 25))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(255, 80, 80)))
                        .rounding(4.0)
                        .inner_margin(6.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("🔴 RECORDING IN PROGRESS").strong().color(Color32::from_rgb(255, 100, 100)));
                                ui.label(RichText::new("Type on your keyboard — keystrokes appear live below. Click Stop when finished.").color(COLOR_TEXT_ACTIVE).small());
                            });
                        });
                } else if let Some(msg) = &state.status_msg {
                    ui.add_space(4.0);
                    ui.label(RichText::new(format!("ℹ {}", msg)).color(COLOR_ACCENT_CYAN).small());
                }

                ui.add_space(4.0);

                // Macro Event sequence canvas
                egui::Frame::none()
                    .fill(COLOR_BG_CANVAS)
                    .stroke(Stroke::new(1.0_f32, COLOR_BORDER))
                    .inner_margin(12.0)
                    .show(ui, |ui| {
                        ui.set_min_height(330.0);
                        if macro_item.actions.is_empty() {
                            ui.centered_and_justified(|ui| {
                                ui.label(RichText::new("No macro actions recorded. Click '⏺ Start' to begin recording keystrokes.").color(COLOR_TEXT_MUTED));
                            });
                        } else {
                            let mut delete_action_idx = None;
                            egui::ScrollArea::vertical()
                                .auto_shrink([false; 2])
                                .stick_to_bottom(state.is_recording)
                                .show(ui, |ui| {
                                    for (idx, action) in macro_item.actions.iter_mut().enumerate() {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(format!("{:02}.", idx + 1)).color(COLOR_TEXT_MUTED).monospace());
                                            match action {
                                                MacroAction::KeyDown(k) => {
                                                    ui.label(RichText::new(format!("↓ Key Down: [{}]", k)).color(COLOR_ACCENT_CYAN).strong());
                                                }
                                                MacroAction::KeyUp(k) => {
                                                    ui.label(RichText::new(format!("↑ Key Up:   [{}]", k)).color(COLOR_TEXT_ACTIVE));
                                                }
                                                MacroAction::Delay(ms) => {
                                                    ui.label(RichText::new("⏱ Delay: ").color(Color32::from_rgb(255, 200, 100)));
                                                    ui.add(egui::DragValue::new(ms).range(1..=10000).speed(1.0).suffix(" ms"));
                                                }
                                            }

                                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                                if ui.small_button("✖").on_hover_text("Delete action").clicked() {
                                                    delete_action_idx = Some(idx);
                                                }
                                            });
                                        });
                                        ui.add_space(2.0);
                                    }
                                });

                            if let Some(del_idx) = delete_action_idx {
                                if del_idx < macro_item.actions.len() {
                                    macro_item.actions.remove(del_idx);
                                }
                            }
                        }
                    });
            }

            if should_save_macros {
                crate::storage::save_macros(&state.macros);
            }
        });
    });
}
