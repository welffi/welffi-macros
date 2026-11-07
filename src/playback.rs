use crate::input;
use crate::log::Log;
use crate::recording::Recording;
use crate::roblox::{self, Hwnd};
use std::collections::BTreeSet;
use std::thread;
use std::time::{Duration, Instant};

const HOVER_MS: u64 = 20;

pub enum Ended {
    Done,
    Stopped,
}

fn scaled(value: i32, scale: f64) -> i32 {
    (value as f64 * scale).round() as i32
}

fn release(keys: &BTreeSet<String>, buttons: &BTreeSet<String>) {
    for key in keys {
        input::key(key, false);
    }
    for button in buttons {
        input::button(button, false);
    }
}

pub fn play(recording: &Recording, window: Hwnd, fallback_size: (i32, i32), log: &Log, stop: &dyn Fn() -> bool) -> Result<Ended, String> {
    if recording.frames.is_empty() {
        return Err("empty recording".to_string());
    }
    let client = roblox::client_rect(window).ok_or("no Roblox window")?;
    if !roblox::activate(window) {
        return Err("couldn't focus Roblox".to_string());
    }
    let (source_w, source_h) = if recording.width > 0 { (recording.width, recording.height) } else { fallback_size };
    let (scale_x, scale_y) = (client.w as f64 / source_w as f64, client.h as f64 / source_h as f64);
    let lost_focus = || !roblox::is_foreground(window);

    let mut keys_down = BTreeSet::new();
    let mut buttons_down = BTreeSet::new();
    let start = Instant::now();
    for frame in &recording.frames {
        let due = start + Duration::from_secs_f64(frame.t.max(0.0) / 1000.0);
        while Instant::now() < due && !stop() && !lost_focus() {
            thread::sleep(Duration::from_millis(1));
        }
        if stop() {
            release(&keys_down, &buttons_down);
            return Ok(Ended::Stopped);
        }
        if lost_focus() {
            release(&keys_down, &buttons_down);
            return Err("Roblox lost focus".to_string());
        }

        let (x, y) = (client.x + scaled(frame.x, scale_x), client.y + scaled(frame.y, scale_y));
        if frame.look == (0, 0) {
            input::move_to(x, y);
        } else {
            input::move_by(frame.look.0, frame.look.1);
        }

        let keys: BTreeSet<String> = frame.keys.iter().cloned().collect();
        let buttons: BTreeSet<String> = frame.buttons.iter().cloned().collect();
        for key in keys.difference(&keys_down) {
            input::key(key, true);
        }
        for key in keys_down.difference(&keys) {
            input::key(key, false);
        }
        for button in buttons.difference(&buttons_down) {
            input::move_to(x, y);
            thread::sleep(Duration::from_millis(HOVER_MS));
            input::button(button, true);
            log.debug(format!("click {},{} -> screen {x},{y}", frame.x, frame.y));
        }
        for button in buttons_down.difference(&buttons) {
            input::button(button, false);
        }
        input::wheel(frame.wheel);
        keys_down = keys;
        buttons_down = buttons;
    }
    release(&keys_down, &buttons_down);
    Ok(Ended::Done)
}
