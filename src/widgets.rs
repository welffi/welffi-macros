use crate::theme::{self, ACCENT, ACCENT_HOVER, ACCENT_INK, DANGER, DANGER_HOVER, FAINT, LINE, MUTED, RAISED, RAISED_HOVER, SURFACE, TEXT};
use eframe::egui::{self, Align2, CornerRadius, CursorIcon, FontId, Response, RichText, Sense, Stroke, StrokeKind, Vec2};

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Accent,
    Plain,
    Danger,
}

pub fn section(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).color(FAINT));
}

pub fn button(ui: &mut egui::Ui, label: &str, kind: Kind, width: f32, height: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let hovered = response.hovered();
    let (fill, text) = match kind {
        Kind::Accent => (if hovered { ACCENT_HOVER } else { ACCENT }, ACCENT_INK),
        Kind::Plain => (if hovered { RAISED_HOVER } else { RAISED }, TEXT),
        Kind::Danger => (if hovered { DANGER_HOVER } else { DANGER }, ACCENT_INK),
    };
    ui.painter().rect_filled(rect, CornerRadius::same(10), fill);
    theme::centered_text(ui.painter(), rect.center(), label, FontId::new(13.5, theme::semibold()), text);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

pub fn button_row(ui: &mut egui::Ui, labels: &[(&str, Kind)], height: f32) -> Option<usize> {
    let gap = 6.0;
    let width = (ui.available_width() - gap * (labels.len() as f32 - 1.0)) / labels.len() as f32;
    let mut clicked = None;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (index, (label, kind)) in labels.iter().enumerate() {
            if button(ui, label, *kind, width, height).clicked() {
                clicked = Some(index);
            }
        }
    });
    clicked
}

pub fn card(ui: &mut egui::Ui, title: &str, about: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 58.0), Sense::click());
    let fill = if response.hovered() { RAISED } else { SURFACE };
    let painter = ui.painter();
    painter.rect(rect, CornerRadius::same(12), fill, Stroke::new(1.0, LINE), StrokeKind::Inside);
    let stripe = egui::Rect::from_min_size(rect.min + Vec2::new(8.0, 16.0), Vec2::new(3.0, rect.height() - 32.0));
    painter.rect_filled(stripe, CornerRadius::same(2), ACCENT);
    painter.text(rect.min + Vec2::new(22.0, 12.0), Align2::LEFT_TOP, title, FontId::new(14.5, theme::semibold()), TEXT);
    painter.text(rect.min + Vec2::new(22.0, 33.0), Align2::LEFT_TOP, about, FontId::proportional(12.0), MUTED);
    response.on_hover_cursor(CursorIcon::PointingHand)
}

pub fn switch(ui: &mut egui::Ui, on: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(40.0, 22.0), Sense::click());
    let progress = ui.ctx().animate_bool(response.id, on);
    let track = if on { ACCENT } else if response.hovered() { RAISED_HOVER } else { RAISED };
    ui.painter().rect(rect, CornerRadius::same(11), track, Stroke::new(1.0, if on { ACCENT } else { LINE }), StrokeKind::Inside);
    let x = egui::lerp(rect.left() + 11.0..=rect.right() - 11.0, progress);
    ui.painter().circle_filled(egui::pos2(x, rect.center().y), 7.0, if on { ACCENT_INK } else { MUTED });
    response.on_hover_cursor(CursorIcon::PointingHand)
}

pub fn divider(ui: &mut egui::Ui) {
    ui.add_space(4.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, LINE);
    ui.add_space(4.0);
}

pub fn value_row(ui: &mut egui::Ui, name: &str, value: &str, color: egui::Color32) {
    ui.horizontal(|ui| {
        ui.add_sized([56.0, 18.0], egui::Label::new(RichText::new(name).size(11.0).color(FAINT)));
        ui.label(RichText::new(value).family(theme::semibold()).size(13.0).color(color));
    });
}
