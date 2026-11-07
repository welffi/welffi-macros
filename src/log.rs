use crate::win;
use eframe::egui;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const KEEP: usize = 200;

#[derive(Clone)]
pub struct Log {
    lines: Arc<Mutex<VecDeque<String>>>,
    debug: Arc<AtomicBool>,
    ctx: egui::Context,
}

impl Log {
    pub fn new(ctx: egui::Context) -> Log {
        Log { lines: Arc::default(), debug: Arc::default(), ctx }
    }

    fn add(&self, tag: &str, text: impl AsRef<str>) {
        let mut lines = self.lines.lock().unwrap();
        lines.push_back(format!("{} {tag}{}", win::clock(), text.as_ref()));
        while lines.len() > KEEP {
            lines.pop_front();
        }
        self.ctx.request_repaint();
    }

    pub fn info(&self, text: impl AsRef<str>) {
        self.add("", text);
    }

    pub fn ok(&self, text: impl AsRef<str>) {
        self.add("[OK] ", text);
    }

    pub fn warn(&self, text: impl AsRef<str>) {
        self.add("[WARN] ", text);
    }

    pub fn error(&self, text: impl AsRef<str>) {
        self.add("[ERR] ", text);
    }

    pub fn debug(&self, text: impl AsRef<str>) {
        if self.debug.load(Ordering::Relaxed) {
            self.add("[DBG] ", text);
        }
    }

    pub fn set_debug(&self, on: bool) {
        self.debug.store(on, Ordering::Relaxed);
    }

    pub fn clear(&self) {
        self.lines.lock().unwrap().clear();
    }

    pub fn lines(&self) -> Vec<String> {
        self.lines.lock().unwrap().iter().cloned().collect()
    }

    pub fn repaint(&self) {
        self.ctx.request_repaint();
    }
}
