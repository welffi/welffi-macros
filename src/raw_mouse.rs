use crate::win;
use std::sync::atomic::{AtomicI32, Ordering};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Input::{GetRawInputData, HRAWINPUT, MOUSE_MOVE_ABSOLUTE, RAWINPUT, RAWINPUTDEVICE, RAWINPUTHEADER, RID_INPUT, RIDEV_INPUTSINK, RIDEV_REMOVE, RIM_TYPEMOUSE, RegisterRawInputDevices};
use windows_sys::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DefWindowProcW, DestroyWindow, HWND_MESSAGE, RegisterClassW, WM_INPUT, WNDCLASSW};

const GENERIC_DESKTOP: u16 = 1;
const MOUSE: u16 = 2;

static MOVED_X: AtomicI32 = AtomicI32::new(0);
static MOVED_Y: AtomicI32 = AtomicI32::new(0);

pub struct RawMouse {
    window: HWND,
}

unsafe extern "system" fn on_message(window: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if message == WM_INPUT {
        let mut input: RAWINPUT = unsafe { std::mem::zeroed() };
        let mut size = size_of::<RAWINPUT>() as u32;
        let read = unsafe { GetRawInputData(lparam as HRAWINPUT, RID_INPUT, (&mut input as *mut RAWINPUT).cast(), &mut size, size_of::<RAWINPUTHEADER>() as u32) };
        if read != u32::MAX && input.header.dwType == RIM_TYPEMOUSE {
            let mouse = unsafe { input.data.mouse };
            if mouse.usFlags & MOUSE_MOVE_ABSOLUTE as u16 == 0 {
                MOVED_X.fetch_add(mouse.lLastX, Ordering::Relaxed);
                MOVED_Y.fetch_add(mouse.lLastY, Ordering::Relaxed);
            }
        }
    }
    unsafe { DefWindowProcW(window, message, wparam, lparam) }
}

fn device(flags: u32, window: HWND) -> RAWINPUTDEVICE {
    RAWINPUTDEVICE { usUsagePage: GENERIC_DESKTOP, usUsage: MOUSE, dwFlags: flags, hwndTarget: window }
}

impl RawMouse {
    pub fn start() -> Option<RawMouse> {
        let class = win::wide("welffi macros raw mouse");
        let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
        let mut info: WNDCLASSW = unsafe { std::mem::zeroed() };
        info.lpfnWndProc = Some(on_message);
        info.hInstance = instance;
        info.lpszClassName = class.as_ptr();
        unsafe { RegisterClassW(&info) };
        let window = unsafe { CreateWindowExW(0, class.as_ptr(), std::ptr::null(), 0, 0, 0, 0, 0, HWND_MESSAGE, std::ptr::null_mut(), instance, std::ptr::null()) };
        if window.is_null() {
            return None;
        }
        let wanted = device(RIDEV_INPUTSINK, window);
        if unsafe { RegisterRawInputDevices(&wanted, 1, size_of::<RAWINPUTDEVICE>() as u32) } == 0 {
            unsafe { DestroyWindow(window) };
            return None;
        }
        MOVED_X.store(0, Ordering::Relaxed);
        MOVED_Y.store(0, Ordering::Relaxed);
        Some(RawMouse { window })
    }

    pub fn take(&self) -> (i32, i32) {
        (MOVED_X.swap(0, Ordering::Relaxed), MOVED_Y.swap(0, Ordering::Relaxed))
    }
}

impl Drop for RawMouse {
    fn drop(&mut self) {
        let removed = device(RIDEV_REMOVE, std::ptr::null_mut());
        unsafe {
            RegisterRawInputDevices(&removed, 1, size_of::<RAWINPUTDEVICE>() as u32);
            DestroyWindow(self.window);
        }
    }
}
