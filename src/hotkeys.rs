use crate::win;
use eframe::egui;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{MOD_CONTROL, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey, VK_ESCAPE, VK_F1, VK_F2, VK_F3};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetMessageW, IsWindowVisible, MSG, PostMessageW, PostThreadMessageW, SW_HIDE, SW_SHOW, SetForegroundWindow, ShowWindow, WM_APP, WM_CLOSE, WM_HOTKEY,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hotkey {
    Start,
    Stop,
    Record,
    Play,
}

pub enum Event {
    Pressed(Hotkey),
    Taken(&'static str),
}

const START: i32 = 1;
const STOP: i32 = 2;
const EXIT: i32 = 3;
const SHOW: i32 = 4;
const RECORD: i32 = 5;
const PLAY: i32 = 6;
const RECORDER_KEYS: u32 = WM_APP + 1;

struct Binding {
    id: i32,
    modifiers: u32,
    vk: u16,
    name: &'static str,
}

const ALWAYS: [Binding; 4] = [
    Binding { id: START, modifiers: 0, vk: VK_F2, name: "F2" },
    Binding { id: STOP, modifiers: 0, vk: VK_F3, name: "F3" },
    Binding { id: EXIT, modifiers: MOD_CONTROL, vk: VK_ESCAPE, name: "Ctrl+Esc" },
    Binding { id: SHOW, modifiers: MOD_CONTROL, vk: VK_F1, name: "Ctrl+F1" },
];

const RECORDER: [Binding; 2] = [
    Binding { id: RECORD, modifiers: MOD_CONTROL, vk: b'R' as u16, name: "Ctrl+R" },
    Binding { id: PLAY, modifiers: MOD_CONTROL, vk: b'P' as u16, name: "Ctrl+P" },
];

pub struct Hotkeys {
    thread: u32,
    events: Receiver<Event>,
}

fn register(binding: &Binding) -> bool {
    unsafe { RegisterHotKey(std::ptr::null_mut(), binding.id, binding.modifiers | MOD_NOREPEAT, binding.vk as u32) != 0 }
}

fn toggle_own_window() {
    let window = win::own_window();
    if window.is_null() {
        return;
    }
    unsafe {
        if IsWindowVisible(window) != 0 {
            ShowWindow(window, SW_HIDE);
        } else {
            ShowWindow(window, SW_SHOW);
            SetForegroundWindow(window);
        }
    }
}

pub fn start(ctx: egui::Context) -> Hotkeys {
    let (events_out, events) = mpsc::channel();
    let (thread_out, thread_in) = mpsc::channel();
    thread::spawn(move || {
        let _ = thread_out.send(unsafe { GetCurrentThreadId() });
        let send = |event| {
            let _ = events_out.send(event);
            ctx.request_repaint();
        };
        for binding in &ALWAYS {
            if !register(binding) {
                send(Event::Taken(binding.name));
            }
        }
        let mut message: MSG = unsafe { std::mem::zeroed() };
        while unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) } > 0 {
            match (message.message, message.wParam as i32) {
                (WM_HOTKEY, START) => send(Event::Pressed(Hotkey::Start)),
                (WM_HOTKEY, STOP) => send(Event::Pressed(Hotkey::Stop)),
                (WM_HOTKEY, RECORD) => send(Event::Pressed(Hotkey::Record)),
                (WM_HOTKEY, PLAY) => send(Event::Pressed(Hotkey::Play)),
                (WM_HOTKEY, SHOW) => toggle_own_window(),
                (WM_HOTKEY, EXIT) => unsafe {
                    PostMessageW(win::own_window(), WM_CLOSE, 0, 0);
                },
                (RECORDER_KEYS, on) => {
                    for binding in &RECORDER {
                        if on != 0 {
                            if !register(binding) {
                                send(Event::Taken(binding.name));
                            }
                        } else {
                            unsafe { UnregisterHotKey(std::ptr::null_mut(), binding.id) };
                        }
                    }
                }
                _ => {}
            }
        }
    });
    Hotkeys { thread: thread_in.recv().unwrap_or(0), events }
}

impl Hotkeys {
    pub fn recorder_keys(&self, on: bool) {
        unsafe { PostThreadMessageW(self.thread, RECORDER_KEYS, on as usize, 0) };
    }

    pub fn next(&self) -> Option<Event> {
        self.events.try_recv().ok()
    }
}
