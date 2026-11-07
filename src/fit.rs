use crate::input;
use crate::roblox::{self, Hwnd, Rect};
use std::thread;
use std::time::Duration;
use windows_sys::Win32::UI::HiDpi::GetDpiForSystem;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GWL_STYLE, GetSystemMetrics, GetWindowLongW, IsZoomed, MoveWindow, SM_CXSCREEN, SM_CYSCREEN, SW_RESTORE, SetWindowLongW, ShowWindow, WS_CAPTION, WS_SYSMENU,
    WS_THICKFRAME,
};

pub type Size = Option<(i32, i32)>;

fn screen() -> (i32, i32) {
    unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) }
}

fn style(window: Hwnd) -> u32 {
    unsafe { GetWindowLongW(window as _, GWL_STYLE) as u32 }
}

fn covers_screen(window: Hwnd) -> bool {
    let outer = roblox::outer_rect(window);
    let (width, height) = screen();
    outer.w >= width && outer.h >= height
}

fn is_fullscreen(window: Hwnd) -> bool {
    style(window) & WS_CAPTION == 0 || covers_screen(window)
}

fn restore_normal(window: Hwnd) {
    for _ in 0..2 {
        let zoomed = unsafe { IsZoomed(window as _) } != 0;
        if !zoomed && !roblox::is_minimized(window) {
            return;
        }
        unsafe { ShowWindow(window as _, SW_RESTORE) };
        roblox::wait_for(Duration::from_millis(250), || unsafe { IsZoomed(window as _) } == 0 && !roblox::is_minimized(window));
    }
}

fn force_windowed(window: Hwnd) -> bool {
    unsafe {
        SetWindowLongW(window as _, GWL_STYLE, (style(window) | WS_CAPTION | WS_THICKFRAME | WS_SYSMENU) as i32);
        thread::sleep(Duration::from_millis(150));
        MoveWindow(window as _, 100, 60, 1000, 700, 1);
    }
    thread::sleep(Duration::from_millis(250));
    !covers_screen(window)
}

fn exit_fullscreen(window: Hwnd) -> bool {
    if !is_fullscreen(window) {
        return true;
    }
    roblox::activate(window);
    for _ in 0..2 {
        if !roblox::is_foreground(window) {
            break;
        }
        input::tap_key("F11");
        if roblox::wait_for(Duration::from_millis(1200), || !is_fullscreen(window)) {
            return true;
        }
    }
    force_windowed(window)
}

fn client_is(window: Hwnd, want: Rect) -> bool {
    roblox::client_rect(window) == Some(want)
}

pub fn fit(window: Hwnd, size: Size) -> Result<String, String> {
    if !roblox::exists(window) {
        return Err("No Roblox window found".to_string());
    }
    let Some((width, height)) = size else {
        return Ok(format!("Left as it is: {}", roblox::describe(window)));
    };
    restore_normal(window);
    if !exit_fullscreen(window) {
        return Err("Couldn't leave fullscreen (F11)".to_string());
    }
    let want = Rect { x: 0, y: 0, w: width, h: height };
    for _ in 0..8 {
        let client = roblox::client_rect(window).unwrap_or_default();
        if client == want {
            break;
        }
        let outer = roblox::outer_rect(window);
        let target = Rect { x: outer.x + want.x - client.x, y: outer.y + want.y - client.y, w: outer.w + want.w - client.w, h: outer.h + want.h - client.h };
        unsafe { MoveWindow(window as _, target.x, target.y, target.w, target.h, 1) };
        roblox::wait_for(Duration::from_millis(140), || client_is(window, want));
    }
    let client = roblox::client_rect(window).unwrap_or_default();
    if (client.w, client.h) != (width, height) {
        let mut message = format!("settled at {}x{}, wanted {width}x{height}", client.w, client.h);
        let dpi = unsafe { GetDpiForSystem() };
        if dpi != 96 && (client.w > width || client.h > height) {
            message += &format!(". That's Roblox's smallest size at {}% scaling. Set display scaling to 100%", dpi * 100 / 96);
        }
        return Err(message);
    }
    Ok(format!("Locked to {}", roblox::describe(window)))
}

pub fn is_fitted(window: Hwnd, size: Size) -> bool {
    match (roblox::client_rect(window), size) {
        (None, _) => false,
        (Some(_), None) => true,
        (Some(client), Some((width, height))) => (client.w, client.h) == (width, height),
    }
}

pub fn fit_all(size: Size) -> (Vec<Hwnd>, Result<String, String>) {
    let mut fitted = Vec::new();
    let mut errors = Vec::new();
    for (index, window) in roblox::find_all().into_iter().enumerate() {
        match fit(window, size) {
            Ok(_) => fitted.push(window),
            Err(message) => errors.push(format!("window {}: {message}", index + 1)),
        }
    }
    for window in fitted.iter().skip(1) {
        roblox::minimize(*window);
    }
    let count = fitted.len();
    let result = if !errors.is_empty() {
        Err(errors.join(", "))
    } else if count == 0 {
        Err("No Roblox window found".to_string())
    } else {
        let at = size.map_or("their own size".to_string(), |(w, h)| format!("{w}x{h}"));
        Ok(format!("{count} window{} at {at}", if count == 1 { "" } else { "s" }))
    };
    (fitted, result)
}
