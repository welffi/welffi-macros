use crate::fit;
use crate::log::Log;
use crate::playback::{self, Ended};
use crate::recording::{self, Recording};
use crate::roblox::{self, Hwnd};
use crate::store;
use crate::vision::{self, Template};
use crate::worker::{self, Cancel, Worker};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const BOX: (i32, i32) = (800, 600);
const TEMPLATES: &str = "ctd";
const TICK_MS: u64 = 300;
const FOCUS_MS: u64 = 150;
const SWAP_MS: u64 = 500;
const WAIT_LOG_MS: u64 = 10_000;
const RESULTS_STEP: usize = 1;

struct Step {
    name: &'static str,
    templates: &'static [&'static str],
    tolerance: i32,
    recording: &'static str,
    label: &'static str,
    hand_off_ms: Option<u64>,
}

const STEPS: [Step; 2] = [
    Step { name: "start_game", templates: &["start_game"], tolerance: 60, recording: "setup_camera", label: "setup camera", hand_off_ms: None },
    Step { name: "results", templates: &["results_lost", "results_won"], tolerance: 60, recording: "restart", label: "restart", hand_off_ms: Some(1500) },
];

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Variant {
    #[default]
    Easy,
    EasyClear,
}

impl Variant {
    pub fn id(self) -> &'static str {
        match self {
            Variant::Easy => "ctd_easy",
            Variant::EasyClear => "ctd_easy_clear",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Variant::Easy => "Easy",
            Variant::EasyClear => "Easy Clear",
        }
    }

    fn recordings(self) -> &'static str {
        match self {
            Variant::Easy => "ctd/easy",
            Variant::EasyClear => "ctd/easy_clear",
        }
    }

    pub fn every_window(self) -> bool {
        self == Variant::Easy
    }

    fn cuts_setup_on_results(self) -> bool {
        self == Variant::EasyClear
    }
}

#[derive(Default)]
pub struct Status {
    pub step: String,
    pub runs: u32,
}

#[derive(Default)]
pub struct Ctd {
    pub variant: Variant,
    worker: Option<Worker>,
    pub status: Arc<Mutex<Status>>,
    pub leniency: Arc<AtomicI32>,
}

struct Client {
    window: Hwnd,
    number: usize,
    step: usize,
    misses: u64,
    fresh: bool,
    state: String,
}

struct Assets {
    templates: Vec<Vec<Template>>,
    recordings: Vec<Recording>,
}

enum Outcome {
    Waiting,
    Played,
    Stop,
}

struct Run {
    variant: Variant,
    clients: Vec<Client>,
    shown: Option<Hwnd>,
    assets: Assets,
    status: Arc<Mutex<Status>>,
    leniency: Arc<AtomicI32>,
    log: Log,
    cancel: Cancel,
}

impl Ctd {
    pub fn running(&self) -> bool {
        worker::is_running(&self.worker)
    }

    pub fn start(&mut self, fitted: Vec<Hwnd>, log: &Log) {
        if self.running() {
            return;
        }
        let (variant, status, leniency, log) = (self.variant, self.status.clone(), self.leniency.clone(), log.clone());
        self.worker = Some(Worker::spawn(move |cancel| {
            let Some(mut run) = Run::prepare(variant, fitted, status, leniency, log.clone(), cancel) else {
                return;
            };
            log.ok(format!("{} started", variant.name()));
            run.go();
            run.status.lock().unwrap().step = "idle".to_string();
            log.info(format!("{} stopped", variant.name()));
        }));
    }

    pub fn stop(&self) {
        if let Some(worker) = &self.worker {
            worker.stop();
        }
    }
}

fn load_assets(variant: Variant, log: &Log) -> Option<Assets> {
    let mut missing = Vec::new();
    let mut assets = Assets { templates: Vec::new(), recordings: Vec::new() };
    for step in &STEPS {
        match recording::load(&store::recording_path(variant.recordings(), step.recording)) {
            Some(take) => {
                log.info(format!("{} - {} frames, {:.1}s", step.recording, take.frames.len(), take.duration() / 1000.0));
                assets.recordings.push(take);
            }
            None => missing.push(format!("{}.txt", step.recording)),
        }
        let mut found = Vec::new();
        for name in step.templates {
            match vision::load(&store::template_path(TEMPLATES, name)) {
                Some(template) => found.push(template),
                None => missing.push(format!("{name}.png")),
            }
        }
        assets.templates.push(found);
    }
    if !missing.is_empty() {
        log.warn(format!("Missing files: {}", missing.join(", ")));
        return None;
    }
    Some(assets)
}

impl Run {
    fn prepare(variant: Variant, mut fitted: Vec<Hwnd>, status: Arc<Mutex<Status>>, leniency: Arc<AtomicI32>, log: Log, cancel: Cancel) -> Option<Run> {
        if !variant.every_window() {
            fitted.truncate(1);
        } else if roblox::find_all().len() != fitted.len() {
            let (windows, result) = fit::fit_all(Some(BOX));
            match result {
                Ok(message) => log.ok(message),
                Err(message) => log.warn(message),
            }
            fitted = windows;
        }
        let mut clients = Vec::new();
        for (index, window) in fitted.into_iter().enumerate() {
            if !roblox::exists(window) {
                continue;
            }
            if !roblox::is_minimized(window) && !fit::is_fitted(window, Some(BOX)) && fit::fit(window, Some(BOX)).is_err() {
                log.error("A Roblox window moved or resized - press Detect + Fit first");
                return None;
            }
            clients.push(Client { window, number: index + 1, step: 0, misses: 0, fresh: true, state: String::new() });
        }
        if clients.is_empty() {
            log.error("No fitted Roblox windows - press Detect + Fit first");
            return None;
        }
        let assets = load_assets(variant, &log)?;
        let mut run = Run { variant, clients, shown: None, assets, status, leniency, log, cancel };
        for index in 0..run.clients.len() {
            run.set_step(index, 0);
            let window = run.clients[index].window;
            if roblox::is_minimized(window) {
                continue;
            }
            if run.shown.is_some() {
                roblox::minimize(window);
            } else {
                run.shown = Some(window);
            }
        }
        let count = run.clients.len();
        run.log.info(format!("Running on {count} window{}", if count == 1 { "" } else { "s" }));
        Some(run)
    }

    fn tag(&self, client: &Client) -> String {
        if self.clients.len() > 1 { format!("[{}] ", client.number) } else { String::new() }
    }

    fn set_state(&mut self, index: usize, state: String) {
        self.clients[index].state = state;
        let line: Vec<String> = self.clients.iter().map(|client| format!("{}{}", self.tag(client), client.state)).collect();
        self.status.lock().unwrap().step = line.join("  ");
        self.log.repaint();
    }

    fn set_step(&mut self, index: usize, step: usize) {
        self.clients[index].step = step;
        self.set_state(index, format!("wait {}", STEPS[step].name));
    }

    fn prune(&mut self) {
        let closed: Vec<String> = self.clients.iter().filter(|client| !roblox::exists(client.window)).map(|client| self.tag(client)).collect();
        for tag in closed {
            self.log.warn(format!("{tag}Roblox window closed"));
        }
        self.clients.retain(|client| roblox::exists(client.window));
    }

    fn go(&mut self) {
        let mut last = 0;
        while self.cancel.nap(TICK_MS) {
            self.prune();
            if self.clients.is_empty() {
                self.log.error("Lost every Roblox window");
                return;
            }
            let count = self.clients.len();
            for offset in 0..count {
                let index = (last + offset) % count;
                match self.try_client(index) {
                    Outcome::Waiting => {}
                    Outcome::Played => {
                        last = index;
                        break;
                    }
                    Outcome::Stop => return,
                }
                if self.cancel.is_set() {
                    return;
                }
            }
        }
    }

    fn show(&mut self, index: usize) -> Result<(), ()> {
        let window = self.clients[index].window;
        if self.shown == Some(window) {
            return Ok(());
        }
        if let Some(shown) = self.shown.filter(|shown| roblox::exists(*shown)) {
            roblox::minimize(shown);
        }
        roblox::show_without_focus(window);
        self.shown = Some(window);
        if !self.cancel.nap(SWAP_MS) {
            return Err(());
        }
        if fit::is_fitted(window, Some(BOX)) {
            return Ok(());
        }
        let tag = self.tag(&self.clients[index]);
        match fit::fit(window, Some(BOX)) {
            Ok(_) => {
                self.log.warn(format!("{tag}Roblox window had resized - refit it"));
                Ok(())
            }
            Err(message) => {
                self.log.error(format!("{tag}Roblox window resized and couldn't be refit - {message}"));
                Err(())
            }
        }
    }

    fn look(&self, index: usize, step: usize) -> Option<vision::Found> {
        let tolerance = vision::tolerance(STEPS[step].tolerance, self.leniency.load(Ordering::Relaxed));
        let templates = &self.assets.templates[step];
        match roblox::client_rect(self.clients[index].window).map(|area| vision::find(area, templates, tolerance)) {
            Some(Ok(found)) => found,
            Some(Err(error)) => {
                self.log.warn(format!("{}{error}", self.tag(&self.clients[index])));
                None
            }
            None => None,
        }
    }

    fn try_client(&mut self, index: usize) -> Outcome {
        if self.show(index).is_err() {
            return Outcome::Stop;
        }
        if std::mem::take(&mut self.clients[index].fresh) && self.clients[index].step != RESULTS_STEP && self.look(index, RESULTS_STEP).is_some() {
            self.log.info(format!("{}Already on results", self.tag(&self.clients[index])));
            self.set_step(index, RESULTS_STEP);
        }
        let window = self.clients[index].window;
        let step = &STEPS[self.clients[index].step];
        let tag = self.tag(&self.clients[index]);
        let template = step.name;
        let Some(found) = self.look(index, self.clients[index].step) else {
            self.clients[index].misses += 1;
            let every = WAIT_LOG_MS / (TICK_MS * self.clients.len() as u64);
            if every > 0 && self.clients[index].misses % every == 0 {
                self.log.info(format!("{tag}Still waiting for {template}"));
            }
            return Outcome::Waiting;
        };

        self.log.ok(format!("{tag}{template} found"));
        self.log.debug(format!("{template} at {},{}", found.x, found.y));
        self.clients[index].misses = 0;
        self.set_state(index, step.label.to_string());
        if !roblox::is_foreground(window) {
            roblox::activate(window);
            if !self.cancel.nap(FOCUS_MS) {
                return Outcome::Stop;
            }
        }

        let hand_off_at = step.hand_off_ms.map(|ms| Instant::now() + Duration::from_millis(ms));
        let handed_off = || hand_off_at.is_some_and(|at| Instant::now() >= at);
        let watch_results = self.variant.cuts_setup_on_results() && self.clients[index].step != RESULTS_STEP;
        let saw_results = AtomicBool::new(false);
        let played = AtomicBool::new(false);
        let results_templates = &self.assets.templates[RESULTS_STEP];
        let results_tolerance = vision::tolerance(STEPS[RESULTS_STEP].tolerance, self.leniency.load(Ordering::Relaxed));
        let cancel = self.cancel.clone();
        let take = &self.assets.recordings[self.clients[index].step];
        let result = std::thread::scope(|scope| {
            if watch_results {
                scope.spawn(|| watch_for_results(window, results_templates, results_tolerance, &played, &saw_results));
            }
            let result = playback::play(take, window, BOX, &self.log, &|| cancel.is_set() || handed_off() || saw_results.load(Ordering::Relaxed));
            played.store(true, Ordering::Relaxed);
            result
        });
        if self.cancel.is_set() {
            return Outcome::Stop;
        }
        if saw_results.load(Ordering::Relaxed) {
            self.log.ok(format!("{tag}results found - cut {} short", step.label));
            self.set_step(index, RESULTS_STEP);
            return Outcome::Played;
        }
        match result {
            Err(message) if !handed_off() => {
                self.log.warn(format!("{tag}{} - {message}", step.label));
                return if roblox::exists(window) { Outcome::Stop } else { Outcome::Played };
            }
            Ok(Ended::Stopped) | Err(_) => self.log.ok(format!("{tag}{} clicked - moving on", step.label)),
            Ok(Ended::Done) => self.log.ok(format!("{tag}{}", step.label)),
        }

        let current = self.clients[index].step;
        if current == STEPS.len() - 1 {
            self.status.lock().unwrap().runs += 1;
        }
        self.set_step(index, (current + 1) % STEPS.len());
        Outcome::Played
    }
}

fn watch_for_results(window: Hwnd, templates: &[Template], tolerance: u8, played: &AtomicBool, seen: &AtomicBool) {
    while !played.load(Ordering::Relaxed) {
        if roblox::client_rect(window).and_then(|area| vision::find(area, templates, tolerance).ok().flatten()).is_some() {
            seen.store(true, Ordering::Relaxed);
            return;
        }
        std::thread::sleep(Duration::from_millis(TICK_MS));
    }
}
