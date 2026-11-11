use crate::win;
use std::collections::HashSet;
use std::thread;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM, POINT, RECT};
use windows_sys::Win32::Graphics::Gdi::ClientToScreen;
use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS};
use windows_sys::Win32::System::ProcessStatus::K32EmptyWorkingSet;
use windows_sys::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_SET_QUOTA, PROCESS_TERMINATE, TerminateProcess};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetClientRect, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, HWND_TOP, IsIconic, IsWindow, IsWindowVisible,
    SW_MINIMIZE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SetForegroundWindow, SetWindowPos, ShowWindow,
};

pub type Hwnd = usize;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

fn handle(window: Hwnd) -> HWND {
    window as HWND
}

fn roblox_pids() -> HashSet<u32> {
    let mut pids = HashSet::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot.is_null() || snapshot as isize == -1 {
            return pids;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32W>() as u32;
        let mut more = Process32FirstW(snapshot, &mut entry) != 0;
        while more {
            let end = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..end]).eq_ignore_ascii_case("RobloxPlayerBeta.exe") {
                pids.insert(entry.th32ProcessID);
            }
            more = Process32NextW(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
    }
    pids
}

fn class_name(window: HWND) -> String {
    let mut buffer = [0u16; 256];
    let length = unsafe { GetClassNameW(window, buffer.as_mut_ptr(), buffer.len() as i32) }.max(0) as usize;
    String::from_utf16_lossy(&buffer[..length])
}

pub fn outer_rect(window: Hwnd) -> Rect {
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    unsafe { GetWindowRect(handle(window), &mut rect) };
    Rect { x: rect.left, y: rect.top, w: rect.right - rect.left, h: rect.bottom - rect.top }
}

pub fn is_minimized(window: Hwnd) -> bool {
    unsafe { IsIconic(handle(window)) != 0 }
}

fn is_game_window(window: Hwnd) -> bool {
    if unsafe { IsWindowVisible(handle(window)) } == 0 {
        return false;
    }
    let size = outer_rect(window);
    is_minimized(window) || (size.w >= 300 && size.h >= 200)
}

struct Search {
    pids: HashSet<u32>,
    by_process: Vec<Hwnd>,
    by_title: Vec<Hwnd>,
}

unsafe extern "system" fn visit(window: HWND, param: LPARAM) -> i32 {
    let search = unsafe { &mut *(param as *mut Search) };
    if is_game_window(window as Hwnd) {
        let mut pid = 0;
        unsafe { GetWindowThreadProcessId(window, &mut pid) };
        if search.pids.contains(&pid) {
            search.by_process.push(window as Hwnd);
        } else if class_name(window) == "WINDOWSCLIENT" && win::window_title(window).to_ascii_lowercase().contains("roblox") {
            search.by_title.push(window as Hwnd);
        }
    }
    1
}

pub fn find_all() -> Vec<Hwnd> {
    let mut search = Search { pids: roblox_pids(), by_process: Vec::new(), by_title: Vec::new() };
    unsafe { EnumWindows(Some(visit), &mut search as *mut Search as LPARAM) };
    let mut found = if search.by_process.is_empty() { search.by_title } else { search.by_process };
    found.sort();
    found
}

pub fn find() -> Option<Hwnd> {
    let score = |window: &Hwnd| {
        let size = outer_rect(*window);
        let bonus = if win::window_title(handle(*window)) == "Roblox" { 2 } else { 1 };
        size.w as i64 * size.h as i64 * bonus
    };
    find_all().into_iter().filter(|window| !is_minimized(*window)).max_by_key(score)
}

pub fn exists(window: Hwnd) -> bool {
    unsafe { IsWindow(handle(window)) != 0 }
}

pub fn client_rect(window: Hwnd) -> Option<Rect> {
    let mut rect: RECT = unsafe { std::mem::zeroed() };
    let mut origin = POINT { x: 0, y: 0 };
    let ok = unsafe { GetClientRect(handle(window), &mut rect) != 0 && ClientToScreen(handle(window), &mut origin) != 0 };
    ok.then_some(Rect { x: origin.x, y: origin.y, w: rect.right, h: rect.bottom })
}

pub fn describe(window: Hwnd) -> String {
    client_rect(window).map_or("not found".to_string(), |r| format!("{}x{} at {},{}", r.w, r.h, r.x, r.y))
}

pub fn is_foreground(window: Hwnd) -> bool {
    unsafe { GetForegroundWindow() as Hwnd == window }
}

pub fn wait_for(timeout: Duration, mut test: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if test() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(15));
    }
}

pub fn activate(window: Hwnd) -> bool {
    if !exists(window) {
        return false;
    }
    unsafe {
        if is_minimized(window) {
            ShowWindow(handle(window), windows_sys::Win32::UI::WindowsAndMessaging::SW_RESTORE);
        }
        let mine = GetCurrentThreadId();
        let theirs = GetWindowThreadProcessId(GetForegroundWindow(), std::ptr::null_mut());
        let attached = theirs != 0 && theirs != mine && AttachThreadInput(mine, theirs, 1) != 0;
        SetForegroundWindow(handle(window));
        if attached {
            AttachThreadInput(mine, theirs, 0);
        }
    }
    wait_for(Duration::from_secs(2), || is_foreground(window))
}

pub fn minimize(window: Hwnd) {
    unsafe { ShowWindow(handle(window), SW_MINIMIZE) };
    thread::spawn(move || {
        thread::sleep(Duration::from_millis(700));
        if exists(window) && is_minimized(window) {
            trim_memory(window);
        }
    });
}

fn trim_memory(window: Hwnd) {
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(handle(window), &mut pid);
        let process = OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA, 0, pid);
        if !process.is_null() {
            K32EmptyWorkingSet(process);
            CloseHandle(process);
        }
    }
}

pub fn close(window: Hwnd) {
    let mut pid = 0;
    unsafe {
        GetWindowThreadProcessId(handle(window), &mut pid);
        let process = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if !process.is_null() {
            TerminateProcess(process, 0);
            CloseHandle(process);
        }
    }
}

pub fn show_without_focus(window: Hwnd) {
    unsafe {
        ShowWindow(handle(window), SW_SHOWNOACTIVATE);
        SetWindowPos(handle(window), HWND_TOP, 0, 0, 0, 0, SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE);
    }
}
