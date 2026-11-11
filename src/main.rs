#![windows_subsystem = "windows"]

mod accounts;
mod app;
mod capture;
mod ctd;
mod disconnect;
mod fit;
mod hotkeys;
mod input;
mod keys;
mod launch;
mod links;
mod log;
mod panels;
mod playback;
mod raw_mouse;
mod recorder;
mod recorder_mode;
mod recording;
mod rejoin;
mod rejoin_panel;
mod roblox;
mod roblox_api;
mod seal;
mod shutdown;
mod status;
mod store;
mod theme;
mod variant;
mod view;
mod vision;
mod widgets;
mod win;
mod worker;

use eframe::egui;

fn main() -> eframe::Result {
    if !win::claim_single_instance() {
        win::focus_running_copy();
        return Ok(());
    }
    win::fine_timers();
    store::seed();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(win::TITLE)
            .with_inner_size([380.0, 720.0])
            .with_min_inner_size([340.0, 560.0])
            .with_always_on_top()
            .with_icon(theme::window_icon()),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let result = eframe::run_native(win::TITLE, options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))));
    win::release_single_instance();
    result
}
