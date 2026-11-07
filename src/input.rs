use crate::keys;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_EXTENDEDKEY, KEYEVENTF_KEYUP, MAPVK_VK_TO_VSC, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_LEFTDOWN,
    MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_MOVE, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEEVENTF_VIRTUALDESK,
    MOUSEEVENTF_WHEEL, MOUSEINPUT, MapVirtualKeyW, SendInput,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN};

const WHEEL_STEP: i32 = 120;

fn send(input: INPUT) {
    unsafe { SendInput(1, &input, size_of::<INPUT>() as i32) };
}

fn mouse(flags: u32, dx: i32, dy: i32, data: i32) {
    let mi = MOUSEINPUT { dx, dy, mouseData: data as _, dwFlags: flags, time: 0, dwExtraInfo: 0 };
    send(INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi } });
}

pub fn normalize(value: i32, origin: i32, span: i32) -> i32 {
    ((value - origin) as i64 * 65535 / (span - 1).max(1) as i64) as i32
}

pub fn move_to(x: i32, y: i32) {
    let (left, top, width, height) =
        unsafe { (GetSystemMetrics(SM_XVIRTUALSCREEN), GetSystemMetrics(SM_YVIRTUALSCREEN), GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN)) };
    mouse(MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK, normalize(x, left, width), normalize(y, top, height), 0);
}

pub fn move_by(dx: i32, dy: i32) {
    mouse(MOUSEEVENTF_MOVE, dx, dy, 0);
}

pub fn button(name: &str, down: bool) {
    let flags = match (name, down) {
        ("Left", true) => MOUSEEVENTF_LEFTDOWN,
        ("Left", false) => MOUSEEVENTF_LEFTUP,
        ("Right", true) => MOUSEEVENTF_RIGHTDOWN,
        ("Right", false) => MOUSEEVENTF_RIGHTUP,
        ("Middle", true) => MOUSEEVENTF_MIDDLEDOWN,
        ("Middle", false) => MOUSEEVENTF_MIDDLEUP,
        _ => return,
    };
    mouse(flags, 0, 0, 0);
}

pub fn wheel(notches: i32) {
    for _ in 0..notches.abs() {
        mouse(MOUSEEVENTF_WHEEL, 0, 0, WHEEL_STEP * notches.signum());
    }
}

pub fn key(name: &str, down: bool) {
    let Some(key) = keys::by_name(name) else {
        return;
    };
    let scan = unsafe { MapVirtualKeyW(key.vk as u32, MAPVK_VK_TO_VSC) } as u16;
    let mut flags = if down { 0 } else { KEYEVENTF_KEYUP };
    if key.extended {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    let ki = KEYBDINPUT { wVk: key.vk, wScan: scan, dwFlags: flags, time: 0, dwExtraInfo: 0 };
    send(INPUT { r#type: INPUT_KEYBOARD, Anonymous: INPUT_0 { ki } });
}

pub fn tap_key(name: &str) {
    key(name, true);
    key(name, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_spans_the_desktop() {
        assert_eq!(normalize(0, 0, 1920), 0);
        assert_eq!(normalize(1919, 0, 1920), 65535);
        assert_eq!(normalize(-1920, -1920, 3840), 0);
    }
}
