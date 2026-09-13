use eframe::egui;
use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use super::theme::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum KeyboardDisplayMode {
    KeyMapping,
    LightEdit,
}

#[derive(Clone, Debug)]
pub struct KeyDef {
    pub id: usize,
    pub name: &'static str,
    pub row: usize,
    pub col_u: f32,
    pub width_u: f32,
    #[allow(dead_code)]
    pub height_u: f32,
}

pub fn get_tkl_87_layout() -> Vec<KeyDef> {
    let mut keys = Vec::with_capacity(87);
    #[allow(unused_assignments)]
    let mut id = 0;

    macro_rules! add_k {
        ($name:expr, $row:expr, $col:expr, $w:expr) => {
            keys.push(KeyDef {
                id,
                name: $name,
                row: $row,
                col_u: $col,
                width_u: $w,
                height_u: 1.0,
            });
            id += 1;
        };
    }

    // Row 0: Function Row (Esc, gap, F1-F4, gap, F5-F8, gap, F9-F12, gap, PrtSc, ScrLk, Pause)
    add_k!("Esc", 0, 0.0, 1.0);
    add_k!("F1", 0, 2.0, 1.0);
    add_k!("F2", 0, 3.0, 1.0);
    add_k!("F3", 0, 4.0, 1.0);
    add_k!("F4", 0, 5.0, 1.0);
    add_k!("F5", 0, 6.5, 1.0);
    add_k!("F6", 0, 7.5, 1.0);
    add_k!("F7", 0, 8.5, 1.0);
    add_k!("F8", 0, 9.5, 1.0);
    add_k!("F9", 0, 11.0, 1.0);
    add_k!("F10", 0, 12.0, 1.0);
    add_k!("F11", 0, 13.0, 1.0);
    add_k!("F12", 0, 14.0, 1.0);
    add_k!("PrtSc", 0, 15.5, 1.0);
    add_k!("ScrLk", 0, 16.5, 1.0);
    add_k!("Pause", 0, 17.5, 1.0);

    // Row 1: Number row (`~`, 1..0, -, =, Backspace, gap, Ins, Home, PgUp)
    let r1_names = ["~", "1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "-", "="];
    let mut c = 0.0;
    for name in r1_names {
        add_k!(name, 1, c, 1.0);
        c += 1.0;
    }
    add_k!("Backspace", 1, c, 2.0);
    add_k!("Ins", 1, 15.5, 1.0);
    add_k!("Home", 1, 16.5, 1.0);
    add_k!("PgUp", 1, 17.5, 1.0);

    // Row 2: Tab, QWERTY, [, ], \, gap, Del, End, PgDn
    add_k!("Tab", 2, 0.0, 1.5);
    c = 1.5;
    let r2_names = ["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P", "[", "]"];
    for name in r2_names {
        add_k!(name, 2, c, 1.0);
        c += 1.0;
    }
    add_k!("\\", 2, c, 1.5);
    add_k!("Del", 2, 15.5, 1.0);
    add_k!("End", 2, 16.5, 1.0);
    add_k!("PgDn", 2, 17.5, 1.0);

    // Row 3: CapsLock, ASDFGHJKL;', Enter
    add_k!("CapsLock", 3, 0.0, 1.75);
    c = 1.75;
    let r3_names = ["A", "S", "D", "F", "G", "H", "J", "K", "L", ";", "'"];
    for name in r3_names {
        add_k!(name, 3, c, 1.0);
        c += 1.0;
    }
    add_k!("Enter", 3, c, 2.25);

    // Row 4: LShift, ZXCVBNM,./, RShift, Up Arrow
    add_k!("Shift", 4, 0.0, 2.25);
    c = 2.25;
    let r4_names = ["Z", "X", "C", "V", "B", "N", "M", "<", ">", "/"];
    for name in r4_names {
        add_k!(name, 4, c, 1.0);
        c += 1.0;
    }
    add_k!("Shift ", 4, c, 2.75);
    add_k!("↑", 4, 16.5, 1.0);

    // Row 5: LCtrl, LWin, LAlt, Space, RAlt, Fn, App, RCtrl, Left, Down, Right
    add_k!("Ctrl", 5, 0.0, 1.25);
    add_k!("Win", 5, 1.25, 1.25);
    add_k!("Alt", 5, 2.5, 1.25);
    add_k!("Space", 5, 3.75, 6.25);
    add_k!("Alt ", 5, 10.0, 1.25);
    add_k!("Fn", 5, 11.25, 1.25);
    add_k!("App", 5, 12.5, 1.25);
    add_k!("Ctrl ", 5, 13.75, 1.25);
    add_k!("←", 5, 15.5, 1.0);
    add_k!("↓", 5, 16.5, 1.0);
    add_k!("→", 5, 17.5, 1.0);

    let _ = id;
    keys
}

pub fn render_keyboard_canvas(
    ui: &mut egui::Ui,
    mode: KeyboardDisplayMode,
    selected_key_id: &mut Option<usize>,
    redefined_keys: &[bool],
    key_colors: &mut [[u8; 3]],
    active_paint_color: [u8; 3],
) -> bool {
    let keys = get_tkl_87_layout();

    let u_px = 30.0;
    let gap_px = 3.5;
    let row_gap_px = 4.0;
    let f_row_gap_px = 8.0;

    let total_width = 18.5 * (u_px + gap_px) + 24.0;
    let total_height = 6.0 * (u_px + row_gap_px) + f_row_gap_px + 24.0;

    let (outer_rect, _resp) = ui.allocate_exact_size(Vec2::new(total_width, total_height), egui::Sense::click_and_drag());
    let painter = ui.painter_at(outer_rect);

    // Chassis background & frame
    let chassis_bg = Color32::from_rgb(20, 24, 26);
    let chassis_border = Color32::from_rgb(45, 58, 65);
    painter.rect_filled(outer_rect, 8.0, chassis_bg);
    painter.rect_stroke(outer_rect, 8.0, Stroke::new(2.0_f32, chassis_border));

    // Two-dot status indicator next to Esc (from screenshot 1 & 3)
    let dot1_pos = Pos2::new(outer_rect.left() + 12.0 + u_px + 10.0, outer_rect.top() + 12.0 + 10.0);
    let dot2_pos = Pos2::new(outer_rect.left() + 12.0 + u_px + 10.0, outer_rect.top() + 12.0 + 18.0);
    painter.circle_filled(dot1_pos, 2.0, Color32::from_gray(100));
    painter.circle_filled(dot2_pos, 2.0, Color32::from_gray(100));

    let mut changed = false;

    let origin = Pos2::new(outer_rect.left() + 12.0, outer_rect.top() + 12.0);

    for k in &keys {
        let y_offset = if k.row == 0 {
            0.0
        } else {
            (u_px + row_gap_px) + f_row_gap_px + (k.row - 1) as f32 * (u_px + row_gap_px)
        };

        let x_offset = k.col_u * (u_px + gap_px);
        let key_w = k.width_u * u_px + (k.width_u - 1.0).max(0.0) * gap_px;
        let key_h = u_px;

        let key_rect = Rect::from_min_size(
            Pos2::new(origin.x + x_offset, origin.y + y_offset),
            Vec2::new(key_w, key_h),
        );

        let key_resp = ui.interact(key_rect, egui::Id::new(("tkl_key", k.id)), egui::Sense::click_and_drag());
        let is_selected = *selected_key_id == Some(k.id);
        let is_redefined = redefined_keys.get(k.id).copied().unwrap_or(false);

        if mode == KeyboardDisplayMode::KeyMapping {
            if key_resp.clicked() {
                *selected_key_id = Some(k.id);
            }

            let bg_col = if is_selected {
                Color32::from_rgb(38, 60, 72)
            } else if key_resp.hovered() {
                Color32::from_rgb(36, 44, 50)
            } else {
                COLOR_KEYCAP_BG
            };

            let stroke_col = if is_selected {
                COLOR_ACCENT_CYAN
            } else if is_redefined {
                COLOR_ACCENT_CYAN
            } else if key_resp.hovered() {
                Color32::from_rgb(70, 88, 100)
            } else {
                COLOR_KEYCAP_BORDER
            };

            let stroke_width = if is_selected { 2.0_f32 } else { 1.0_f32 };

            painter.rect_filled(key_rect, 3.5, bg_col);
            painter.rect_stroke(key_rect, 3.5, Stroke::new(stroke_width, stroke_col));

            // Key label
            let text_color = if is_selected || is_redefined {
                COLOR_ACCENT_CYAN
            } else {
                COLOR_KEYCAP_TEXT
            };

            let font_size = if k.name.len() > 4 { 10.0 } else { 12.0 };
            let text_pos = Pos2::new(key_rect.center().x, key_rect.center().y - 1.0);
            painter.text(
                text_pos,
                egui::Align2::CENTER_CENTER,
                k.name.trim(),
                egui::FontId::proportional(font_size),
                text_color,
            );

            // Redefined cyan dot badge
            if is_redefined {
                let badge_pos = Pos2::new(key_rect.right() - 4.0, key_rect.top() + 4.0);
                painter.circle_filled(badge_pos, 2.0, COLOR_ACCENT_CYAN);
            }
        } else {
            // LightEdit Mode (Wireframe per-key illumination, matching Screenshot 3)
            let current_rgb = key_colors.get(k.id).copied().unwrap_or([0, 0, 0]);
            let is_lit = current_rgb != [0, 0, 0];

            if key_resp.clicked() || (key_resp.dragged() && key_resp.hovered()) {
                if k.id < key_colors.len() {
                    key_colors[k.id] = active_paint_color;
                    *selected_key_id = Some(k.id);
                    changed = true;
                }
            }

            let bg_col = if is_lit {
                Color32::from_rgb(current_rgb[0], current_rgb[1], current_rgb[2])
            } else {
                Color32::from_rgb(12, 15, 18)
            };

            let stroke_col = if is_lit {
                Color32::WHITE
            } else if key_resp.hovered() {
                COLOR_ACCENT_CYAN
            } else {
                Color32::from_rgb(60, 75, 85)
            };

            painter.rect_filled(key_rect, 4.0, bg_col);
            painter.rect_stroke(key_rect, 4.0, Stroke::new(1.0_f32, stroke_col));

            // In Screenshot 3, Wireframe labels are subtly visible or minimalist
            let label_col = if is_lit {
                // Invert if bright background
                let brightness = current_rgb[0] as f32 * 0.299 + current_rgb[1] as f32 * 0.587 + current_rgb[2] as f32 * 0.114;
                if brightness > 140.0 { Color32::BLACK } else { Color32::WHITE }
            } else {
                Color32::from_gray(80)
            };

            let font_size = if k.name.len() > 4 { 9.0 } else { 11.0 };
            painter.text(
                key_rect.center(),
                egui::Align2::CENTER_CENTER,
                k.name.trim(),
                egui::FontId::proportional(font_size),
                label_col,
            );
        }
    }

    changed
}

pub fn key_id_to_vk(id: usize) -> Option<u8> {
    match id {
        0 => Some(0x1B), // Esc
        1..=12 => Some(0x70 + (id - 1) as u8), // F1-F12
        13 => Some(0x2C), // PrtSc
        14 => Some(0x91), // ScrLk
        15 => Some(0x13), // Pause
        16 => Some(0xC0), // ~
        17..=25 => Some(0x31 + (id - 17) as u8), // 1-9
        26 => Some(0x30), // 0
        27 => Some(0xBD), // -
        28 => Some(0xBB), // =
        29 => Some(0x08), // Backspace
        30 => Some(0x2D), // Ins
        31 => Some(0x24), // Home
        32 => Some(0x21), // PgUp
        33 => Some(0x09), // Tab
        34 => Some(0x51), // Q
        35 => Some(0x57), // W
        36 => Some(0x45), // E
        37 => Some(0x52), // R
        38 => Some(0x54), // T
        39 => Some(0x59), // Y
        40 => Some(0x55), // U
        41 => Some(0x49), // I
        42 => Some(0x4F), // O
        43 => Some(0x50), // P
        44 => Some(0xDB), // [
        45 => Some(0xDD), // ]
        46 => Some(0xDC), // \
        47 => Some(0x2E), // Del
        48 => Some(0x23), // End
        49 => Some(0x22), // PgDn
        50 => Some(0x14), // CapsLock
        51 => Some(0x41), // A
        52 => Some(0x53), // S
        53 => Some(0x44), // D
        54 => Some(0x46), // F
        55 => Some(0x47), // G
        56 => Some(0x48), // H
        57 => Some(0x4A), // J
        58 => Some(0x4B), // K
        59 => Some(0x4C), // L
        60 => Some(0xBA), // ;
        61 => Some(0xDE), // '
        62 => Some(0x0D), // Enter
        63 => Some(0xA0), // LShift
        64 => Some(0x5A), // Z
        65 => Some(0x58), // X
        66 => Some(0x43), // C
        67 => Some(0x56), // V
        68 => Some(0x42), // B
        69 => Some(0x4E), // N
        70 => Some(0x4D), // M
        71 => Some(0xBC), // <
        72 => Some(0xBE), // >
        73 => Some(0xBF), // /
        74 => Some(0xA1), // RShift
        75 => Some(0x26), // Up Arrow (↑)
        76 => Some(0xA2), // LCtrl
        77 => Some(0x5B), // LWin
        78 => Some(0xA4), // LAlt
        79 => Some(0x20), // Space
        80 => Some(0xA5), // RAlt
        81 => None,       // Fn (internal)
        82 => Some(0x5D), // App
        83 => Some(0xA3), // RCtrl
        84 => Some(0x25), // Left Arrow (←)
        85 => Some(0x28), // Down Arrow (↓)
        86 => Some(0x27), // Right Arrow (→)
        _ => None,
    }
}
