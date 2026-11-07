use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, FontId, IconData, Painter, Pos2, Stroke};
use std::sync::Arc;

pub const INK: Color32 = Color32::from_rgb(0x0F, 0x10, 0x13);
pub const SURFACE: Color32 = Color32::from_rgb(0x17, 0x19, 0x1E);
pub const RAISED: Color32 = Color32::from_rgb(0x1F, 0x22, 0x29);
pub const RAISED_HOVER: Color32 = Color32::from_rgb(0x2D, 0x30, 0x39);
pub const LINE: Color32 = Color32::from_rgb(0x26, 0x29, 0x31);
pub const TEXT: Color32 = Color32::from_rgb(0xF1, 0xEF, 0xEA);
pub const MUTED: Color32 = Color32::from_rgb(0x9A, 0x9F, 0xAA);
pub const FAINT: Color32 = Color32::from_rgb(0x6A, 0x6F, 0x7A);
pub const ACCENT: Color32 = Color32::from_rgb(0xF2, 0xB5, 0x44);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(0xFF, 0xCB, 0x66);
pub const ACCENT_LINE: Color32 = Color32::from_rgb(0x5C, 0x48, 0x1E);
pub const ACCENT_INK: Color32 = Color32::from_rgb(0x20, 0x17, 0x03);
pub const DANGER: Color32 = Color32::from_rgb(0xF0, 0x7A, 0x72);
pub const DANGER_HOVER: Color32 = Color32::from_rgb(0xFF, 0x9A, 0x92);
pub const GOOD: Color32 = Color32::from_rgb(0x6A, 0xD3, 0x9A);

pub fn centered_text(painter: &Painter, center: Pos2, text: impl ToString, font: FontId, color: Color32) {
    let galley = painter.layout_no_wrap(text.to_string(), font, color);
    let origin = center - galley.mesh_bounds.center().to_vec2();
    painter.galley(origin, galley, color);
}

pub fn semibold() -> FontFamily {
    FontFamily::Name("semibold".into())
}

pub fn apply(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let fallback = fonts.families[&FontFamily::Proportional].clone();
    add_system_font(&mut fonts, "segoeui.ttf", FontFamily::Proportional);
    fonts.families.insert(semibold(), fallback);
    add_system_font(&mut fonts, "seguisb.ttf", semibold());
    add_system_font(&mut fonts, "consola.ttf", FontFamily::Monospace);
    ctx.set_fonts(fonts);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = INK;
    visuals.override_text_color = Some(TEXT);
    visuals.extreme_bg_color = RAISED;
    visuals.selection.bg_fill = ACCENT_LINE;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, LINE);
    visuals.widgets.inactive.weak_bg_fill = RAISED;
    visuals.widgets.hovered.weak_bg_fill = RAISED_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, RAISED_HOVER);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT_LINE);
    visuals.window_fill = SURFACE;
    visuals.window_stroke = Stroke::new(1.0, LINE);
    ctx.set_visuals(visuals);
}

fn add_system_font(fonts: &mut FontDefinitions, file: &str, family: FontFamily) {
    let Ok(bytes) = std::fs::read(std::path::Path::new(r"C:\Windows\Fonts").join(file)) else {
        return;
    };
    fonts.font_data.insert(file.to_string(), Arc::new(FontData::from_owned(bytes)));
    fonts.families.entry(family).or_default().insert(0, file.to_string());
}

pub fn window_icon() -> IconData {
    IconData { rgba: include_bytes!("../assets/icon64.rgba").to_vec(), width: 64, height: 64 }
}
