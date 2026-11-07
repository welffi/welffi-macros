use std::sync::atomic::{AtomicUsize, Ordering};
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, SYSTEMTIME};
use windows_sys::Win32::Media::timeBeginPeriod;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;
use windows_sys::Win32::System::Threading::{CreateMutexW, GetCurrentProcessId};
use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsIconic, SW_RESTORE, SetForegroundWindow, ShowWindow};

pub const TITLE: &str = "welffi macros";

pub fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

static INSTANCE: AtomicUsize = AtomicUsize::new(0);

pub fn claim_single_instance() -> bool {
    let name = wide("welffi_macros_single_instance");
    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 1, name.as_ptr());
        INSTANCE.store(handle as usize, Ordering::SeqCst);
        GetLastError() != ERROR_ALREADY_EXISTS
    }
}

pub fn release_single_instance() {
    let handle = INSTANCE.swap(0, Ordering::SeqCst);
    if handle != 0 {
        unsafe { CloseHandle(handle as HANDLE) };
    }
}

pub fn fine_timers() {
    unsafe { timeBeginPeriod(1) };
}

pub fn window_title(window: HWND) -> String {
    let mut buffer = [0u16; 256];
    let length = unsafe { GetWindowTextW(window, buffer.as_mut_ptr(), buffer.len() as i32) }.max(0) as usize;
    String::from_utf16_lossy(&buffer[..length])
}

struct TitleSearch {
    pid: u32,
    title: &'static str,
    only_mine: bool,
    found: usize,
}

unsafe extern "system" fn visit(window: HWND, param: LPARAM) -> i32 {
    let search = unsafe { &mut *(param as *mut TitleSearch) };
    let mut pid = 0;
    unsafe { GetWindowThreadProcessId(window, &mut pid) };
    let owner_ok = if search.only_mine { pid == search.pid } else { pid != search.pid };
    if owner_ok && window_title(window) == search.title {
        search.found = window as usize;
        return 0;
    }
    1
}

fn find_titled(only_mine: bool) -> HWND {
    let mut search = TitleSearch { pid: unsafe { GetCurrentProcessId() }, title: TITLE, only_mine, found: 0 };
    unsafe { EnumWindows(Some(visit), &mut search as *mut TitleSearch as LPARAM) };
    search.found as HWND
}

pub fn own_window() -> HWND {
    find_titled(true)
}

pub fn focus_running_copy() {
    let window = find_titled(false);
    if window.is_null() {
        return;
    }
    unsafe {
        if IsIconic(window) != 0 {
            ShowWindow(window, SW_RESTORE);
        }
        SetForegroundWindow(window);
    }
}

pub fn clock() -> String {
    let mut now: SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { GetLocalTime(&mut now) };
    format!("{:02}:{:02}:{:02}", now.wHour, now.wMinute, now.wSecond)
}
