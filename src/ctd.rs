mod assets;
mod recover;
mod retry;

use assets::{Assets, STEPS, load_assets};
use crate::disconnect;
use crate::fit;
use crate::log::Log;
use crate::playback::{self, Ended};
use crate::rejoin::Target;
use crate::roblox::{self, Hwnd};
use crate::shutdown::{self, Shutdown};
use crate::status::Status;
use crate::variant::Variant;
use crate::vision::{self, Template};
use crate::worker::{self, Cancel, Worker};
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const BOX: (i32, i32) = (800, 600);
const TICK_MS: u64 = 300;
const FOCUS_MS: u64 = 150;
const SWAP_MS: u64 = 500;
const WAIT_LOG_MS: u64 = 10_000;
const RESULTS_STEP: usize = 1;
const WON_TEMPLATE: &str = "results_won";

#[derive(Default)]
pub struct Ctd {
    pub variant: Variant,
    worker: Option<Worker>,
    pub status: Arc<Mutex<Status>>,
    pub leniency: Arc<AtomicI32>,
    pub shutdown: Shutdown,
    pub rejoin: Option<Target>,
}

struct Client {
    window: Hwnd,
    number: usize,
    step: usize,
    misses: u64,
    fresh: bool,
    next_disconnect_check: Instant,
    state: String,
    quiet_since: Instant,
    recovering: Option<Instant>,
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
    shutdown: Shutdown,
    target: Option<Target>,
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
        let (shutdown, target) = (self.shutdown.clone(), self.rejoin.clone());
        self.worker = Some(Worker::spawn(move |cancel| {
            *status.lock().unwrap() = Status::default();
            let Some(mut run) = Run::prepare(variant, fitted, status, leniency, shutdown, target, log.clone(), cancel) else {
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

impl Run {
    fn prepare(variant: Variant, mut fitted: Vec<Hwnd>, status: Arc<Mutex<Status>>, leniency: Arc<AtomicI32>, shutdown: Shutdown, target: Option<Target>, log: Log, cancel: Cancel) -> Option<Run> {
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
            clients.push(Client { window, number: index + 1, step: 0, misses: 0, fresh: true, next_disconnect_check: Instant::now(), state: String::new(), quiet_since: Instant::now(), recovering: None });
        }
        if clients.is_empty() {
            log.error("No fitted Roblox windows - press Detect + Fit first");
            return None;
        }
        let target = target.filter(|_| {
            let single = clients.len() == 1;
            if !single {
                log.warn("Rejoin needs a single Roblox window - leaving it off");
            }
            single
        });
        let assets = load_assets(variant, &log)?;
        if (shutdown.on_disconnect.load(Ordering::Relaxed) || target.is_some()) && assets.disconnected.is_none() {
            log.warn("Missing global/disconnected.png - can't watch for disconnects");
        }
        let mut run = Run { variant, clients, shown: None, assets, status, leniency, shutdown, target, log, cancel };
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
        self.clients[index].quiet_since = Instant::now();
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
        self.search(index, &self.assets.templates[step], STEPS[step].tolerance)
    }

    fn search(&self, index: usize, templates: &[Template], base_tolerance: i32) -> Option<vision::Found> {
        let tolerance = vision::tolerance(base_tolerance, self.leniency.load(Ordering::Relaxed));
        match roblox::client_rect(self.clients[index].window).map(|area| vision::find(area, templates, tolerance)) {
            Some(Ok(found)) => found,
            Some(Err(error)) => {
                self.log.warn(format!("{}{error}", self.tag(&self.clients[index])));
                None
            }
            None => None,
        }
    }

    fn disconnected(&mut self, index: usize) -> bool {
        let Some(template) = &self.assets.disconnected else {
            return false;
        };
        let watching = self.shutdown.on_disconnect.load(Ordering::Relaxed) || self.target.is_some();
        let client = &mut self.clients[index];
        if !watching || !disconnect::due(&mut client.next_disconnect_check) {
            return false;
        }
        disconnect::showing(client.window, template, self.leniency.load(Ordering::Relaxed))
    }

    fn handle_disconnect(&mut self, index: usize) -> Outcome {
        let tag = self.tag(&self.clients[index]);
        if self.target.is_some() {
            self.log.warn(format!("{tag}Disconnected - rejoining"));
            return self.rejoin(index);
        }
        shutdown::begin(&format!("{tag}Disconnected"), &self.log);
        Outcome::Stop
    }

    fn try_client(&mut self, index: usize) -> Outcome {
        if self.show(index).is_err() {
            return Outcome::Stop;
        }
        if self.disconnected(index) {
            return self.handle_disconnect(index);
        }
        if self.clients[index].recovering.is_some() {
            return self.recover(index);
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
            if self.clients[index].step != RESULTS_STEP && self.cancel_showing(index) {
                return self.retry_restart(index);
            }
            if self.in_lobby(index) {
                self.begin_recovery(index);
            }
            return Outcome::Waiting;
        };

        self.log.ok(format!("{tag}{template} found"));
        self.log.debug(format!("{template} at {},{}", found.x, found.y));
        if self.clients[index].step == RESULTS_STEP {
            let won = STEPS[RESULTS_STEP].templates[found.template] == WON_TEMPLATE;
            self.log.info(format!("{tag}{}", if won { "Won" } else { "Lost" }));
            self.status.lock().unwrap().record(won);
        }
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
            let runs = {
                let mut status = self.status.lock().unwrap();
                status.runs += 1;
                status.runs
            };
            if self.shutdown.runs_done(runs) {
                shutdown::begin(&format!("{runs} runs done"), &self.log);
                return Outcome::Stop;
            }
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
