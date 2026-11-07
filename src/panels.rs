use crate::app::App;
use crate::recorder_mode::{Act, RATES, SIZES};
use crate::recording;
use crate::theme::{self, ACCENT, DANGER, GOOD, TEXT};
use crate::widgets::{self, Kind};
use eframe::egui::{self, RichText};
use std::sync::atomic::Ordering;

fn picker(ui: &mut egui::Ui, id: &str, labels: &[&str], selected: usize) -> Option<usize> {
    let mut choice = selected.min(labels.len() - 1);
    egui::ComboBox::from_id_salt(id).width(ui.available_width()).selected_text(labels[choice]).show_ui(ui, |ui| {
        for (index, label) in labels.iter().enumerate() {
            ui.selectable_value(&mut choice, index, *label);
        }
    });
    (choice != selected).then_some(choice)
}

impl App {
    pub fn ctd_panel(&mut self, ui: &mut egui::Ui) {
        widgets::section(ui, "SETTINGS");
        ui.horizontal(|ui| {
            if widgets::switch(ui, self.settings.debug).clicked() {
                self.set_debug(!self.settings.debug);
            }
            ui.label(RichText::new("Debug mode").size(12.5));
        });
        ui.add_space(6.0);

        let mut leniency = self.ctd.leniency.load(Ordering::Relaxed);
        ui.horizontal(|ui| {
            widgets::section(ui, "LENIENCY");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(leniency.to_string()).family(theme::semibold()).size(12.0).color(ACCENT));
            });
        });
        ui.spacing_mut().slider_width = ui.available_width();
        let slider = ui.add(egui::Slider::new(&mut leniency, 0..=100).show_value(false));
        if slider.changed() {
            self.set_leniency(leniency);
        }

        widgets::divider(ui);
        let (step, runs) = {
            let status = self.ctd.status.lock().unwrap();
            let step = if self.ctd.running() && !status.step.is_empty() { status.step.clone() } else { "idle".to_string() };
            (step, status.runs)
        };
        widgets::value_row(ui, "STEP", &step, TEXT);
        widgets::value_row(ui, "RUNS", &runs.to_string(), TEXT);
        ui.add_space(8.0);

        let (label, kind) = if self.ctd.running() { ("Stop", Kind::Danger) } else { ("Start", Kind::Accent) };
        if widgets::button(ui, label, kind, ui.available_width(), 34.0).clicked() {
            if self.ctd.running() { self.stop() } else { self.start() }
        }
    }

    pub fn recorder_panel(&mut self, ui: &mut egui::Ui) {
        let idle = self.recorder.act() == Act::Idle;
        widgets::section(ui, "ROBLOX SIZE");
        let sizes: Vec<&str> = SIZES.iter().map(|(label, _)| *label).collect();
        if let Some(choice) = ui.add_enabled_ui(idle, |ui| picker(ui, "size", &sizes, self.settings.recorder_size)).inner {
            self.settings.recorder_size = choice;
            self.settings.save();
            self.log.info(format!("Roblox size set to {} - press Detect + Fit", sizes[choice]));
        }
        ui.add_space(6.0);
        widgets::section(ui, "CAPTURE RATE");
        let rates: Vec<&str> = RATES.iter().map(|(label, _)| *label).collect();
        if let Some(choice) = ui.add_enabled_ui(idle, |ui| picker(ui, "rate", &rates, self.settings.recorder_rate)).inner {
            self.settings.recorder_rate = choice;
            self.settings.save();
        }

        widgets::divider(ui);
        let (state, color) = match self.recorder.act() {
            Act::Idle => ("idle", TEXT),
            Act::Recording => ("RECORDING", DANGER),
            Act::Playing => ("PLAYING", GOOD),
        };
        let (frames, time) = {
            let take = self.recorder.take.lock().unwrap();
            (take.frames.len(), recording::clock_text(take.duration()))
        };
        widgets::value_row(ui, "STATE", state, color);
        widgets::value_row(ui, "TIME", &time, TEXT);
        widgets::value_row(ui, "FRAMES", &frames.to_string(), TEXT);
        ui.add_space(8.0);

        let act = self.recorder.act();
        let record = if act == Act::Recording { ("Stop", Kind::Danger) } else { ("Record", Kind::Accent) };
        let play = if act == Act::Playing { ("Stop", Kind::Danger) } else { ("Play", Kind::Plain) };
        match widgets::button_row(ui, &[record, play], 32.0) {
            Some(0) => self.toggle_record(),
            Some(_) => self.toggle_play(),
            None => {}
        }
        ui.add_space(4.0);
        match widgets::button_row(ui, &[("Save", Kind::Plain), ("Load", Kind::Plain)], 28.0) {
            Some(0) => self.recorder.save(&self.log),
            Some(_) => self.recorder.load(&self.log),
            None => {}
        }
    }
}
