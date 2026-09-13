use eframe::egui;
use egui::{Color32, Pos2, Rect, Stroke, Vec2};
use super::theme::*;

pub const PRESET_SWATCHES: [[u8; 3]; 16] = [
    [255, 0, 0],     // Red
    [255, 127, 0],   // Orange
    [255, 255, 0],   // Yellow
    [139, 69, 19],   // Brown
    [0, 255, 0],     // Green
    [127, 255, 0],   // Lime
    [0, 0, 255],     // Blue
    [138, 43, 226],  // Purple
    [0, 191, 255],   // Sky Blue
    [32, 178, 170],  // Light Teal
    [144, 238, 144], // Pale Green
    [169, 169, 169], // Silver Grey
    [112, 128, 144], // Slate
    [64, 64, 64],    // Dark Grey
    [255, 255, 255], // White
    [224, 224, 224], // Off White
];

pub struct ColorPickerState {
    pub is_open: bool,
    pub hue: f32, // 0.0 .. 1.0
    pub sat: f32, // 0.0 .. 1.0
    pub val: f32, // 0.0 .. 1.0
    pub hex_input: String,
    pub r_input: u8,
    pub g_input: u8,
    pub b_input: u8,
}

impl Default for ColorPickerState {
    fn default() -> Self {
        Self {
            is_open: false,
            hue: 0.0,
            sat: 0.0,
            val: 1.0,
            hex_input: "FFFFFF".to_string(),
            r_input: 255,
            g_input: 255,
            b_input: 255,
        }
    }
}

impl ColorPickerState {
    pub fn set_rgb(&mut self, r: u8, g: u8, b: u8) {
        self.r_input = r;
        self.g_input = g;
        self.b_input = b;
        self.hex_input = format!("{:02X}{:02X}{:02X}", r, g, b);

        let rf = r as f32 / 255.0;
        let gf = g as f32 / 255.0;
        let bf = b as f32 / 255.0;

        let max = rf.max(gf).max(bf);
        let min = rf.min(gf).min(bf);
        let delta = max - min;

        self.val = max;
        self.sat = if max == 0.0 { 0.0 } else { delta / max };

        if delta == 0.0 {
            self.hue = 0.0;
        } else if max == rf {
            self.hue = (((gf - bf) / delta) % 6.0) / 6.0;
            if self.hue < 0.0 { self.hue += 1.0; }
        } else if max == gf {
            self.hue = (((bf - rf) / delta) + 2.0) / 6.0;
        } else {
            self.hue = (((rf - gf) / delta) + 4.0) / 6.0;
        }
    }

    pub fn current_rgb(&self) -> [u8; 3] {
        [self.r_input, self.g_input, self.b_input]
    }

    fn sync_hsv_to_rgb(&mut self) {
        let h = self.hue * 6.0;
        let c = self.val * self.sat;
        let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
        let m = self.val - c;

        let (r1, g1, b1) = if h < 1.0 {
            (c, x, 0.0)
        } else if h < 2.0 {
            (x, c, 0.0)
        } else if h < 3.0 {
            (0.0, c, x)
        } else if h < 4.0 {
            (0.0, x, c)
        } else if h < 5.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        self.r_input = ((r1 + m) * 255.0).clamp(0.0, 255.0) as u8;
        self.g_input = ((g1 + m) * 255.0).clamp(0.0, 255.0) as u8;
        self.b_input = ((b1 + m) * 255.0).clamp(0.0, 255.0) as u8;
        self.hex_input = format!("{:02X}{:02X}{:02X}", self.r_input, self.g_input, self.b_input);
    }
}

pub fn show_color_picker_popup(
    ui: &mut egui::Ui,
    state: &mut ColorPickerState,
    on_color_change: &mut dyn FnMut([u8; 3]),
) {
    if !state.is_open {
        return;
    }

    egui::Frame::window(ui.style())
        .fill(Color32::from_rgb(24, 34, 40))
        .stroke(Stroke::new(1.5_f32, COLOR_BORDER))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(240.0);

            // Header with Close button
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🎨 Custom Color").strong().color(COLOR_TEXT_ACTIVE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(egui::RichText::new("✖ Close").color(COLOR_ACCENT_CYAN).small()).on_hover_text("Close color picker").clicked() {
                        state.is_open = false;
                    }
                });
            });
            ui.separator();
            ui.add_space(4.0);

            // 1. 2D Saturation / Value Gradient Canvas
            let (rect_sv, resp_sv) = ui.allocate_exact_size(Vec2::new(236.0, 130.0), egui::Sense::click_and_drag());
            if ui.is_rect_visible(rect_sv) {
                let painter = ui.painter();
                // Draw SV gradient
                let base_color = hsv_to_color32(state.hue, 1.0, 1.0);
                let mesh = egui::Mesh {
                    indices: vec![0, 1, 2, 0, 2, 3],
                    vertices: vec![
                        egui::epaint::Vertex { pos: rect_sv.left_top(), uv: Pos2::ZERO, color: Color32::WHITE },
                        egui::epaint::Vertex { pos: rect_sv.right_top(), uv: Pos2::ZERO, color: base_color },
                        egui::epaint::Vertex { pos: rect_sv.right_bottom(), uv: Pos2::ZERO, color: Color32::BLACK },
                        egui::epaint::Vertex { pos: rect_sv.left_bottom(), uv: Pos2::ZERO, color: Color32::BLACK },
                    ],
                    texture_id: egui::TextureId::default(),
                };
                painter.add(egui::Shape::mesh(mesh));

                // Handle position
                let handle_pos = Pos2::new(
                    rect_sv.left() + state.sat * rect_sv.width(),
                    rect_sv.top() + (1.0 - state.val) * rect_sv.height(),
                );
                painter.circle_stroke(handle_pos, 5.0, Stroke::new(2.0_f32, Color32::WHITE));
                painter.circle_stroke(handle_pos, 6.0, Stroke::new(1.0_f32, Color32::BLACK));
            }

            if resp_sv.clicked() || resp_sv.dragged() {
                if let Some(pos) = resp_sv.interact_pointer_pos() {
                    let s = ((pos.x - rect_sv.left()) / rect_sv.width()).clamp(0.0, 1.0);
                    let v = (1.0 - (pos.y - rect_sv.top()) / rect_sv.height()).clamp(0.0, 1.0);
                    state.sat = s;
                    state.val = v;
                    state.sync_hsv_to_rgb();
                    on_color_change(state.current_rgb());
                }
            }

            ui.add_space(8.0);

            // 2. Rainbow Hue Spectrum Bar
            let (rect_hue, resp_hue) = ui.allocate_exact_size(Vec2::new(236.0, 14.0), egui::Sense::click_and_drag());
            if ui.is_rect_visible(rect_hue) {
                let painter = ui.painter();
                let steps = 12;
                let step_w = rect_hue.width() / steps as f32;
                for i in 0..steps {
                    let h1 = i as f32 / steps as f32;
                    let c1 = hsv_to_color32(h1, 1.0, 1.0);
                    let sub_rect = Rect::from_min_max(
                        Pos2::new(rect_hue.left() + i as f32 * step_w, rect_hue.top()),
                        Pos2::new(rect_hue.left() + (i + 1) as f32 * step_w, rect_hue.bottom()),
                    );
                    painter.rect_filled(sub_rect, 0.0, c1);
                }
                // Hue selector line
                let handle_x = rect_hue.left() + state.hue * rect_hue.width();
                painter.rect_filled(
                    Rect::from_min_max(Pos2::new(handle_x - 2.0, rect_hue.top() - 1.0), Pos2::new(handle_x + 2.0, rect_hue.bottom() + 1.0)),
                    1.0,
                    Color32::WHITE,
                );
            }

            if resp_hue.clicked() || resp_hue.dragged() {
                if let Some(pos) = resp_hue.interact_pointer_pos() {
                    state.hue = ((pos.x - rect_hue.left()) / rect_hue.width()).clamp(0.0, 1.0);
                    state.sync_hsv_to_rgb();
                    on_color_change(state.current_rgb());
                }
            }

            ui.add_space(8.0);

            // 3. Hex & RGB Numeric Inputs
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    let mut hex = state.hex_input.clone();
                    if ui.add(egui::TextEdit::singleline(&mut hex).desired_width(70.0)).changed() {
                        if hex.len() <= 6 {
                            state.hex_input = hex.to_uppercase();
                            if let Ok(val) = u32::from_str_radix(&state.hex_input, 16) {
                                if state.hex_input.len() == 6 {
                                    let r = ((val >> 16) & 0xFF) as u8;
                                    let g = ((val >> 8) & 0xFF) as u8;
                                    let b = (val & 0xFF) as u8;
                                    state.set_rgb(r, g, b);
                                    on_color_change(state.current_rgb());
                                }
                            }
                        }
                    }
                    ui.label(egui::RichText::new("Hex").small().color(COLOR_TEXT_MUTED));
                });

                ui.add_space(4.0);

                ui.vertical(|ui| {
                    if ui.add(egui::DragValue::new(&mut state.r_input).range(0..=255).speed(1.0)).changed() {
                        let [_, g, b] = state.current_rgb();
                        state.set_rgb(state.r_input, g, b);
                        on_color_change(state.current_rgb());
                    }
                    ui.label(egui::RichText::new("R").small().color(COLOR_TEXT_MUTED));
                });

                ui.vertical(|ui| {
                    if ui.add(egui::DragValue::new(&mut state.g_input).range(0..=255).speed(1.0)).changed() {
                        let [r, _, b] = state.current_rgb();
                        state.set_rgb(r, state.g_input, b);
                        on_color_change(state.current_rgb());
                    }
                    ui.label(egui::RichText::new("G").small().color(COLOR_TEXT_MUTED));
                });

                ui.vertical(|ui| {
                    if ui.add(egui::DragValue::new(&mut state.b_input).range(0..=255).speed(1.0)).changed() {
                        let [r, g, _] = state.current_rgb();
                        state.set_rgb(r, g, state.b_input);
                        on_color_change(state.current_rgb());
                    }
                    ui.label(egui::RichText::new("B").small().color(COLOR_TEXT_MUTED));
                });
            });

            ui.add_space(6.0);

            // 4. Quick Swatches: 2 rows of 8
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    for i in 0..8 {
                        let rgb = PRESET_SWATCHES[i];
                        let c = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                        let (rect_sw, resp_sw) = ui.allocate_exact_size(Vec2::new(24.0, 18.0), egui::Sense::click());
                        ui.painter().rect_filled(rect_sw, 2.0, c);
                        ui.painter().rect_stroke(rect_sw, 2.0, Stroke::new(1.0_f32, Color32::from_gray(60)));
                        if resp_sw.clicked() {
                            state.set_rgb(rgb[0], rgb[1], rgb[2]);
                            on_color_change(state.current_rgb());
                        }
                    }
                });
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    for i in 8..16 {
                        let rgb = PRESET_SWATCHES[i];
                        let c = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                        let (rect_sw, resp_sw) = ui.allocate_exact_size(Vec2::new(24.0, 18.0), egui::Sense::click());
                        ui.painter().rect_filled(rect_sw, 2.0, c);
                        ui.painter().rect_stroke(rect_sw, 2.0, Stroke::new(1.0_f32, Color32::from_gray(60)));
                        if resp_sw.clicked() {
                            state.set_rgb(rgb[0], rgb[1], rgb[2]);
                            on_color_change(state.current_rgb());
                        }
                    }
                });
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("✔ Apply & Close").strong().color(COLOR_ACCENT_CYAN)).clicked() {
                    state.is_open = false;
                }
            });
        });
}

fn hsv_to_color32(h: f32, s: f32, v: f32) -> Color32 {
    let h6 = (h * 6.0) % 6.0;
    let c = v * s;
    let x = c * (1.0 - ((h6 % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = if h6 < 1.0 {
        (c, x, 0.0)
    } else if h6 < 2.0 {
        (x, c, 0.0)
    } else if h6 < 3.0 {
        (0.0, c, x)
    } else if h6 < 4.0 {
        (0.0, x, c)
    } else if h6 < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    Color32::from_rgb(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
