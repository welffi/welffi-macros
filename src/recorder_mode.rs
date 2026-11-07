use crate::fit::{self, Size};
use crate::log::Log;
use crate::playback::{self, Ended};
use crate::recorder;
use crate::recording::{self, Recording};
use crate::roblox::{self, Hwnd};
use crate::store;
use crate::worker::{self, Worker};
use std::sync::{Arc, Mutex};

pub const SIZES: [(&str, Size); 4] = [("1280 x 720", Some((1280, 720))), ("1024 x 600", Some((1024, 600))), ("800 x 600", Some((800, 600))), ("Don't resize", None)];
pub const RATES: [(&str, u32); 3] = [("62 fps (16ms)", 16), ("125 fps (8ms)", 8), ("250 fps (4ms)", 4)];

#[derive(Clone, Copy, PartialEq)]
pub enum Act {
    Idle,
    Recording,
    Playing,
}

pub struct RecorderMode {
    pub take: Arc<Mutex<Recording>>,
    worker: Option<Worker>,
    act: Act,
}

impl Default for RecorderMode {
    fn default() -> RecorderMode {
        RecorderMode { take: Arc::default(), worker: None, act: Act::Idle }
    }
}

pub fn size(index: usize) -> Size {
    SIZES.get(index).map_or(SIZES[0].1, |(_, size)| *size)
}

pub fn rate(index: usize) -> u32 {
    RATES.get(index).map_or(RATES[1].1, |(_, ms)| *ms)
}

fn summary(take: &Recording) -> String {
    format!("{} frames, {}", take.frames.len(), recording::clock_text(take.duration()))
}

impl RecorderMode {
    pub fn act(&self) -> Act {
        if worker::is_running(&self.worker) { self.act } else { Act::Idle }
    }

    pub fn stop(&self) {
        if let Some(worker) = &self.worker {
            worker.stop();
        }
    }

    pub fn toggle_record(&mut self, window: Option<Hwnd>, size: Size, poll_ms: u32, log: &Log) {
        match self.act() {
            Act::Recording => return self.stop(),
            Act::Playing => return log.warn("Stop playback first"),
            Act::Idle => {}
        }
        let Some(window) = window.filter(|window| fit::is_fitted(*window, size)) else {
            let have = window.and_then(roblox::client_rect).map_or("not found".to_string(), |r| format!("{}x{}", r.w, r.h));
            return log.error(format!("Roblox is {have} - press Detect + Fit first"));
        };
        let (take, log) = (self.take.clone(), log.clone());
        self.act = Act::Recording;
        self.worker = Some(Worker::spawn(move |cancel| {
            let area = roblox::describe(window);
            log.ok(format!("Recording {area} at {poll_ms}ms - Stop (or F3) when done"));
            match recorder::record(window, poll_ms, &take, &cancel) {
                Ok(()) => log.info(format!("Stopped - {}", summary(&take.lock().unwrap()))),
                Err(message) => log.error(format!("Could not start recording - {message}")),
            }
        }));
    }

    pub fn toggle_play(&mut self, window: Option<Hwnd>, size: Size, log: &Log) {
        match self.act() {
            Act::Playing => return self.stop(),
            Act::Recording => return log.warn("Stop recording first"),
            Act::Idle => {}
        }
        let take = self.take.lock().unwrap().clone();
        if take.frames.is_empty() {
            return log.warn("Nothing to play - record or load first");
        }
        let Some(window) = window.filter(|window| fit::is_fitted(*window, size)) else {
            return log.warn("Detect + Fit first");
        };
        let fallback = roblox::client_rect(window).map_or((1280, 720), |r| (r.w, r.h));
        let log = log.clone();
        self.act = Act::Playing;
        self.worker = Some(Worker::spawn(move |cancel| {
            log.ok("Looping playback - Stop (or F3) to end");
            let mut loops = 0;
            loop {
                match playback::play(&take, window, fallback, &log, &|| cancel.is_set()) {
                    Ok(Ended::Done) => {
                        loops += 1;
                        log.info(format!("Loop {loops} done"));
                    }
                    Ok(Ended::Stopped) => break,
                    Err(message) => {
                        log.warn(format!("Playback - {message}"));
                        break;
                    }
                }
            }
            log.info("Playback stopped");
        }));
    }

    pub fn save(&self, log: &Log) {
        if self.act() == Act::Recording {
            return log.warn("Stop recording first");
        }
        let take = self.take.lock().unwrap().clone();
        if take.frames.is_empty() {
            return log.warn("Nothing recorded");
        }
        let picked = rfd::FileDialog::new().set_title("Save recording").set_directory(store::recordings_folder()).set_file_name("recording.txt").add_filter("Macro", &["txt"]).save_file();
        let Some(mut path) = picked else {
            return;
        };
        if path.extension().is_none_or(|extension| !extension.eq_ignore_ascii_case("txt")) {
            path.as_mut_os_string().push(".txt");
        }
        match std::fs::write(&path, recording::to_text(&take)) {
            Ok(()) => log.ok(format!("Saved {}", path.display())),
            Err(error) => log.error(format!("Save failed - {error}")),
        }
    }

    pub fn load(&mut self, log: &Log) {
        if self.act() != Act::Idle {
            return;
        }
        let picked = rfd::FileDialog::new().set_title("Load recording").set_directory(store::recordings_folder()).add_filter("Macro", &["txt"]).pick_file();
        let Some(path) = picked else {
            return;
        };
        match recording::load(&path) {
            Some(take) if !take.frames.is_empty() => {
                log.ok(format!("Loaded {} from {}", summary(&take), path.display()));
                *self.take.lock().unwrap() = take;
            }
            _ => log.error("Load failed"),
        }
    }
}
