use crate::app::{App, Page};
use crate::ctd::Variant;
use crate::fit;
use crate::roblox;
use crate::theme::{self, ACCENT, DANGER, FAINT, GOOD, INK, LINE, MUTED, RAISED, TEXT};
use crate::widgets::{self, Kind};
use eframe::egui::{self, CornerRadius, CursorIcon, FontId, Margin, RichText, Sense, Stroke, Vec2};

const PAD: i8 = 18;

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(INK).inner_margin(Margin::same(PAD))).show(ui, |ui| match self.page {
            Page::Home => self.home(ui),
            page => self.mode_page(ui, page),
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.stop();
    }
}

fn title(ui: &mut egui::Ui, text: &str) {
    ui.label(RichText::new(text).family(theme::semibold()).size(20.0).color(TEXT));
}

fn back_button(ui: &mut egui::Ui) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(28.0), Sense::click());
    let fill = if response.hovered() { RAISED } else { INK };
    ui.painter().rect(rect, CornerRadius::same(8), fill, Stroke::new(1.0, LINE), egui::StrokeKind::Inside);
    theme::centered_text(ui.painter(), rect.center(), "‹", FontId::proportional(18.0), MUTED);
    response.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

fn bind_row(ui: &mut egui::Ui, key: &str, action: &str) {
    ui.horizontal(|ui| {
        ui.add_sized([72.0, 18.0], egui::Label::new(RichText::new(key).family(theme::semibold()).size(12.5).color(ACCENT)));
        ui.label(RichText::new(action).size(12.5).color(MUTED));
    });
}

fn log_color(line: &str) -> egui::Color32 {
    if line.contains("[ERR]") {
        DANGER
    } else if line.contains("[WARN]") {
        ACCENT
    } else if line.contains("[OK]") {
        GOOD
    } else if line.contains("[DBG]") {
        FAINT
    } else {
        MUTED
    }
}

impl App {
    fn home(&mut self, ui: &mut egui::Ui) {
        title(ui, "welffi macros");
        ui.label(RichText::new("Pick a mode").size(12.5).color(MUTED));
        ui.add_space(16.0);
        widgets::section(ui, "CTD");
        if widgets::card(ui, "Easy", "Runs the easy loop on every Roblox window").clicked() {
            self.open(Page::Ctd(Variant::Easy));
        }
        ui.add_space(8.0);
        if widgets::card(ui, "Easy Clear", "One window - setup stops as soon as results show").clicked() {
            self.open(Page::Ctd(Variant::EasyClear));
        }
        ui.add_space(14.0);
        widgets::section(ui, "TOOLS");
        if widgets::card(ui, "Recorder", "Record your inputs and play them back").clicked() {
            self.open(Page::Recorder);
        }
    }

    fn mode_page(&mut self, ui: &mut egui::Ui, page: Page) {
        ui.horizontal(|ui| {
            if back_button(ui) {
                self.close_mode();
            }
            ui.add_space(4.0);
            let (game, name) = match page {
                Page::Ctd(variant) => ("CTD", variant.name()),
                _ => ("Tools", "Recorder"),
            };
            ui.label(RichText::new(game).size(13.0).color(FAINT));
            title(ui, name);
        });
        ui.add_space(12.0);
        self.window_status(ui);
        widgets::divider(ui);
        match page {
            Page::Ctd(_) => self.ctd_panel(ui),
            _ => self.recorder_panel(ui),
        }
        widgets::divider(ui);
        self.keys(ui, page);
        widgets::divider(ui);
        self.log_box(ui);
    }

    fn window_status(&mut self, ui: &mut egui::Ui) {
        widgets::section(ui, "ROBLOX WINDOW");
        let size = self.box_size();
        let (color, text) = match self.window.filter(|window| roblox::exists(*window)) {
            None => (DANGER, "not found".to_string()),
            Some(window) if fit::is_fitted(window, size) => (GOOD, roblox::describe(window)),
            Some(window) => (ACCENT, format!("{} - needs a fit", roblox::describe(window))),
        };
        ui.horizontal(|ui| {
            let (dot, _) = ui.allocate_exact_size(Vec2::new(10.0, 30.0), Sense::hover());
            ui.painter().circle_filled(dot.center(), 4.0, color);
            ui.label(RichText::new(text).size(12.5).color(TEXT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if self.is_fitting() { "Fitting..." } else { "Detect + Fit" };
                if widgets::button(ui, label, Kind::Plain, 110.0, 30.0).clicked() {
                    self.detect_and_fit();
                }
            });
        });
    }

    fn keys(&self, ui: &mut egui::Ui, page: Page) {
        widgets::section(ui, "KEYS");
        let binds: &[(&str, &str)] = match page {
            Page::Recorder => &[("Ctrl+R", "Record"), ("Ctrl+P", "Play (loop)"), ("F3", "Stop")],
            _ => &[("F2", "Start / stop"), ("F3", "Stop")],
        };
        for (key, action) in binds.iter().chain(&[("Ctrl+F1", "Show / hide"), ("Ctrl+Esc", "Exit")]) {
            bind_row(ui, key, action);
        }
    }

    fn log_box(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            widgets::section(ui, "LOG");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(egui::Label::new(RichText::new("Clear").size(11.0).color(FAINT)).sense(Sense::click())).on_hover_cursor(CursorIcon::PointingHand).clicked() {
                    self.log.clear();
                }
            });
        });
        egui::Frame::NONE.fill(RAISED).corner_radius(CornerRadius::same(10)).inner_margin(Margin::same(10)).show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false, false]).stick_to_bottom(true).show(ui, |ui| {
                for line in self.log.lines() {
                    ui.label(RichText::new(&line).monospace().size(11.5).color(log_color(&line)));
                }
            });
        });
    }
}
