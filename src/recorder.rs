use crate::keys::{self, Key};
use crate::raw_mouse::RawMouse;
use crate::recording::{Frame, Recording};
use crate::roblox::{self, Hwnd};
use crate::worker::Cancel;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_LBUTTON, VK_MBUTTON, VK_RBUTTON};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetCursorPos, MSG, MSLLHOOKSTRUCT, PM_REMOVE, PeekMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx, WH_MOUSE_LL,
    WM_MOUSEWHEEL,
};

const DIP_PX: i32 = 12;
const SKIPPED: [&str; 5] = ["F2", "F3", "Escape", "LControl", "RControl"];
const CTRL_COMBOS: [&str; 2] = ["r", "p"];
const BUTTONS: [(u16, &str); 3] = [(VK_LBUTTON, "Left"), (VK_RBUTTON, "Right"), (VK_MBUTTON, "Middle")];

static WHEEL: AtomicI32 = AtomicI32::new(0);

unsafe extern "system" fn on_mouse(code: i32, message: WPARAM, data: LPARAM) -> LRESULT {
    if code >= 0 && message == WM_MOUSEWHEEL as WPARAM {
        let event = unsafe { &*(data as *const MSLLHOOKSTRUCT) };
        let delta = (event.mouseData >> 16) as u16 as i16;
        WHEEL.fetch_add(delta.signum() as i32, Ordering::Relaxed);
    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, message, data) }
}

#[derive(Default)]
struct Dip {
    was_held: bool,
    anchor: (i32, i32),
    previous: (i32, i32),
}

impl Dip {
    fn settle(&mut self, at: (i32, i32), held: bool) -> (i32, i32) {
        if held && !self.was_held {
            self.anchor = self.previous;
        }
        self.was_held = held;
        if !held {
            self.previous = at;
            return at;
        }
        let near = (at.0 - self.anchor.0).abs() <= DIP_PX && (at.1 - self.anchor.1).abs() <= DIP_PX;
        if near { self.anchor } else { at }
    }
}

fn held_look(cursor: (i32, i32), last_cursor: (i32, i32), moved: (i32, i32)) -> (i32, i32) {
    if cursor == last_cursor { moved } else { (0, 0) }
}

fn is_down(vk: u16) -> bool {
    unsafe { GetAsyncKeyState(vk as i32) as u16 & 0x8000 != 0 }
}

fn pump() {
    let mut message: MSG = unsafe { std::mem::zeroed() };
    while unsafe { PeekMessageW(&mut message, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}

fn keys_down(watched: &[Key]) -> Vec<String> {
    let ctrl = is_down(VK_CONTROL);
    watched.iter().filter(|key| is_down(key.vk) && !(ctrl && CTRL_COMBOS.contains(&key.name))).map(|key| key.name.to_string()).collect()
}

pub fn record(window: Hwnd, poll_ms: u32, take: &Arc<Mutex<Recording>>, cancel: &Cancel) -> Result<(), String> {
    let origin = roblox::client_rect(window).ok_or("no Roblox window")?;
    *take.lock().unwrap() = Recording { poll_ms, width: origin.w, height: origin.h, frames: Vec::new() };
    let watched: Vec<Key> = keys::recordable().into_iter().filter(|key| !SKIPPED.contains(&key.name)).collect();
    WHEEL.store(0, Ordering::Relaxed);
    let hook = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(on_mouse), GetModuleHandleW(std::ptr::null()), 0) };

    let start = Instant::now();
    let mut dip = Dip::default();
    let raw = RawMouse::start();
    let mut last_cursor = None;
    let mut tick = 0;
    while !cancel.is_set() {
        pump();
        let t = start.elapsed().as_secs_f64() * 1000.0;
        let mut cursor = POINT { x: 0, y: 0 };
        unsafe { GetCursorPos(&mut cursor) };
        let (x, y) = (cursor.x - origin.x, cursor.y - origin.y);
        let moved = raw.as_ref().map_or((0, 0), RawMouse::take);
        let look = held_look((x, y), last_cursor.unwrap_or((x, y)), moved);
        last_cursor = Some((x, y));
        let in_box = x >= 0 && y >= 0 && x <= origin.w && y <= origin.h;
        let buttons: Vec<String> = if in_box { BUTTONS.iter().filter(|(vk, _)| is_down(*vk)).map(|(_, name)| name.to_string()).collect() } else { Vec::new() };
        let (x, y) = dip.settle((x, y), !buttons.is_empty());
        let frame = Frame { x, y, keys: keys_down(&watched), buttons, wheel: WHEEL.swap(0, Ordering::Relaxed), look, t };
        take.lock().unwrap().frames.push(frame);

        tick += 1;
        let next = start + Duration::from_millis(tick * poll_ms as u64);
        while Instant::now() < next && !cancel.is_set() {
            pump();
            thread::sleep(Duration::from_millis(1));
        }
    }
    if !hook.is_null() {
        unsafe { UnhookWindowsHookEx(hook) };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_moves_while_held_stay_on_the_press_spot() {
        let mut dip = Dip::default();
        assert_eq!(dip.settle((100, 100), false), (100, 100));
        assert_eq!(dip.settle((104, 98), true), (100, 100));
        assert_eq!(dip.settle((112, 88), true), (100, 100));
        assert_eq!(dip.settle((113, 100), true), (113, 100));
        assert_eq!(dip.settle((50, 50), false), (50, 50));
        assert_eq!(dip.settle((52, 52), true), (50, 50));
    }

    #[test]
    fn mouse_moves_count_as_look_only_while_the_cursor_is_held() {
        assert_eq!(held_look((401, 299), (401, 299), (-6, 14)), (-6, 14));
        assert_eq!(held_look((420, 310), (401, 299), (19, 11)), (0, 0));
        assert_eq!(held_look((401, 299), (401, 299), (0, 0)), (0, 0));
    }
}
