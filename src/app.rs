use crate::accounts::{Account, Server};
use crate::ctd::{self, Ctd};
use crate::variant::Variant;
use crate::fit::{self, Size};
use crate::hotkeys::{self, Event, Hotkey, Hotkeys};
use crate::log::Log;
use crate::recorder_mode::{self, RecorderMode};
use crate::rejoin_panel::AddServer;
use crate::roblox::{self, Hwnd};
use crate::store::Settings;
use crate::theme;
use eframe::egui;
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};

const PANEL_GAP: i32 = 10;

#[derive(Clone, Copy, PartialEq)]
pub enum Page {
    Home,
    Ctd(Variant),
    Recorder,
}

type FitResult = (Vec<Hwnd>, Result<String, String>);

pub struct App {
    pub log: Log,
    pub settings: Settings,
    pub page: Page,
    pub window: Option<Hwnd>,
    pub ctd: Ctd,
    pub recorder: RecorderMode,
    pub accounts: Vec<Account>,
    pub servers: Vec<Server>,
    pub add_server: AddServer,
    hotkeys: Hotkeys,
    fitted: Vec<Hwnd>,
    fitting: Option<JoinHandle<FitResult>>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext) -> App {
        theme::apply(&cc.egui_ctx);
        let log = Log::new(cc.egui_ctx.clone());
        let settings = Settings::load();
        log.set_debug(settings.debug);
        let ctd = Ctd::default();
        ctd.shutdown.on_disconnect.store(settings.shut_down_on_disconnect, Ordering::Relaxed);
        ctd.shutdown.after_runs.store(settings.shut_down_after_runs, Ordering::Relaxed);
        let hotkeys = hotkeys::start(cc.egui_ctx.clone());
        let mut app = App { log, settings, page: Page::Home, window: None, ctd, recorder: RecorderMode::default(), accounts: Vec::new(), servers: Vec::new(), add_server: AddServer::default(), hotkeys, fitted: Vec::new(), fitting: None };
        app.refresh_rejoin();
        app
    }

    pub fn box_size(&self) -> Size {
        match self.page {
            Page::Recorder => recorder_mode::size(self.settings.recorder_size),
            _ => Some(ctd::BOX),
        }
    }

    pub fn open(&mut self, page: Page) {
        self.page = page;
        if let Page::Ctd(variant) = page {
            self.ctd.variant = variant;
            self.ctd.leniency.store(self.settings.leniency(variant.id()), Ordering::Relaxed);
            *self.ctd.status.lock().unwrap() = Default::default();
        }
        self.refresh_rejoin();
        self.hotkeys.recorder_keys(page == Page::Recorder);
        self.window = roblox::find();
        self.fitted = self.window.into_iter().collect();
    }

    pub fn close_mode(&mut self) {
        self.stop();
        self.hotkeys.recorder_keys(false);
        self.page = Page::Home;
    }

    pub fn is_fitting(&self) -> bool {
        self.fitting.is_some()
    }

    pub fn is_busy(&self) -> bool {
        self.ctd.running() || self.recorder.act() != recorder_mode::Act::Idle
    }

    pub fn detect_and_fit(&mut self) {
        if self.is_fitting() || self.is_busy() {
            return;
        }
        let size = self.box_size();
        let every_window = matches!(self.page, Page::Ctd(variant) if variant.every_window());
        self.log.info("Detecting Roblox");
        self.fitting = Some(thread::spawn(move || {
            if every_window {
                return fit::fit_all(size);
            }
            match roblox::find() {
                Some(window) => (vec![window], fit::fit(window, size)),
                None => (Vec::new(), Err("No Roblox window found".to_string())),
            }
        }));
    }

    fn finish_fit(&mut self, ctx: &egui::Context) {
        if !self.fitting.as_ref().is_some_and(JoinHandle::is_finished) {
            return;
        }
        let Some(Ok((windows, result))) = self.fitting.take().map(JoinHandle::join) else {
            return;
        };
        match result {
            Ok(message) => self.log.ok(message),
            Err(message) => self.log.error(message),
        }
        self.window = windows.first().copied().or_else(roblox::find);
        self.fitted = windows;
        if let Some(client) = self.window.and_then(roblox::client_rect) {
            let x = (client.x + client.w + PANEL_GAP) as f32 / ctx.pixels_per_point();
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, 0.0)));
        }
    }

    pub fn start(&mut self) {
        if matches!(self.page, Page::Ctd(_)) && !self.ctd.running() && !self.is_fitting() {
            self.ctd.start(self.fitted.clone(), &self.log);
        }
    }

    pub fn stop(&mut self) {
        self.ctd.stop();
        self.recorder.stop();
    }

    pub fn toggle_record(&mut self) {
        let poll_ms = recorder_mode::rate(self.settings.recorder_rate);
        self.recorder.toggle_record(self.window, self.box_size(), poll_ms, &self.log);
    }

    pub fn toggle_play(&mut self) {
        self.recorder.toggle_play(self.window, self.box_size(), &self.log);
    }

    fn on_hotkey(&mut self, hotkey: Hotkey) {
        match (self.page, hotkey) {
            (_, Hotkey::Stop) => self.stop(),
            (Page::Ctd(_), Hotkey::Start) if self.ctd.running() => self.stop(),
            (Page::Ctd(_), Hotkey::Start) => self.start(),
            (Page::Recorder, Hotkey::Start | Hotkey::Record) => self.toggle_record(),
            (Page::Recorder, Hotkey::Play) => self.toggle_play(),
            _ => {}
        }
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        while let Some(event) = self.hotkeys.next() {
            match event {
                Event::Pressed(hotkey) => self.on_hotkey(hotkey),
                Event::Taken(name) => self.log.warn(format!("{name} is in use by another program")),
            }
        }
        self.finish_fit(ctx);
        if self.is_fitting() || self.is_busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        } else if self.page != Page::Home {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
    }

    pub fn set_leniency(&mut self, value: i32) {
        self.ctd.leniency.store(value, Ordering::Relaxed);
        self.settings.leniency.insert(self.ctd.variant.id().to_string(), value);
        self.settings.save();
    }

    pub fn set_shut_down_on_disconnect(&mut self, on: bool) {
        self.settings.shut_down_on_disconnect = on;
        self.settings.save();
        self.ctd.shutdown.on_disconnect.store(on, Ordering::Relaxed);
        self.log.info(format!("Shut down on disconnect {}", if on { "on" } else { "off" }));
    }

    pub fn set_shut_down_after_runs(&mut self, runs: u32) {
        self.settings.shut_down_after_runs = runs;
        self.settings.save();
        self.ctd.shutdown.after_runs.store(runs, Ordering::Relaxed);
    }

    pub fn set_debug(&mut self, on: bool) {
        self.settings.debug = on;
        self.settings.save();
        self.log.set_debug(on);
        self.log.info(format!("Debug mode {}", if on { "on" } else { "off" }));
    }
}
