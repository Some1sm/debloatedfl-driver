use eframe::egui;
use egui::{Color32, Stroke};

pub const COLOR_BG_MAIN: Color32 = Color32::from_rgb(26, 38, 44);
pub const COLOR_BG_CANVAS: Color32 = Color32::from_rgb(20, 30, 35);
pub const COLOR_BG_PANEL: Color32 = Color32::from_rgb(34, 49, 57);
pub const COLOR_BORDER: Color32 = Color32::from_rgb(44, 65, 76);
pub const COLOR_ACCENT_CYAN: Color32 = Color32::from_rgb(0, 229, 201);
pub const COLOR_TEXT_ACTIVE: Color32 = Color32::from_rgb(240, 246, 250);
pub const COLOR_TEXT_MUTED: Color32 = Color32::from_rgb(138, 159, 168);

pub const COLOR_KEYCAP_BG: Color32 = Color32::from_rgb(28, 34, 38);
pub const COLOR_KEYCAP_BORDER: Color32 = Color32::from_rgb(52, 64, 72);
pub const COLOR_KEYCAP_TEXT: Color32 = Color32::from_rgb(215, 225, 230);
#[allow(dead_code)]
pub const COLOR_KEYCAP_SELECTED: Color32 = Color32::from_rgb(0, 229, 201);

pub fn apply_theme(ctx: &egui::Context, font_size_delta: f32) {
    let mut style = (*ctx.style()).clone();
    style.visuals.dark_mode = true;
    style.visuals.override_text_color = Some(COLOR_TEXT_ACTIVE);
    style.visuals.panel_fill = COLOR_BG_MAIN;
    style.visuals.window_fill = COLOR_BG_MAIN;
    
    // Widgets styling
    style.visuals.widgets.noninteractive.bg_fill = COLOR_BG_PANEL;
    style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_MUTED);

    style.visuals.widgets.inactive.bg_fill = COLOR_BG_PANEL;
    style.visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, COLOR_BORDER);
    style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_ACTIVE);

    style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(42, 60, 70);
    style.visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, COLOR_ACCENT_CYAN);
    style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, COLOR_TEXT_ACTIVE);

    style.visuals.widgets.active.bg_fill = Color32::from_rgb(48, 70, 82);
    style.visuals.widgets.active.bg_stroke = Stroke::new(1.5_f32, COLOR_ACCENT_CYAN);
    style.visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);

    // Font styles scaling
    style.text_styles.insert(
        egui::TextStyle::Small,
        egui::FontId::proportional(11.0 + font_size_delta),
    );
    style.text_styles.insert(
        egui::TextStyle::Body,
        egui::FontId::proportional(14.0 + font_size_delta),
    );
    style.text_styles.insert(
        egui::TextStyle::Button,
        egui::FontId::proportional(14.0 + font_size_delta),
    );
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::proportional(18.0 + font_size_delta),
    );
    style.text_styles.insert(
        egui::TextStyle::Monospace,
        egui::FontId::monospace(13.0 + font_size_delta),
    );

    ctx.set_style(style);
}
