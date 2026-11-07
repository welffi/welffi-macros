use crate::roblox::Rect;
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BitBlt, CAPTUREBLT, CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SRCCOPY, SelectObject,
};

pub struct Shot {
    pub width: usize,
    pub height: usize,
    pub bgra: Vec<u8>,
}

pub fn grab(area: Rect) -> Option<Shot> {
    if area.w <= 0 || area.h <= 0 {
        return None;
    }
    let (width, height) = (area.w as usize, area.h as usize);
    let mut bgra = vec![0u8; width * height * 4];
    unsafe {
        let screen = GetDC(std::ptr::null_mut());
        let memory = CreateCompatibleDC(screen);
        let bitmap = CreateCompatibleBitmap(screen, area.w, area.h);
        let previous = SelectObject(memory, bitmap);
        let copied = BitBlt(memory, 0, 0, area.w, area.h, screen, area.x, area.y, SRCCOPY | CAPTUREBLT) != 0;
        SelectObject(memory, previous);
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader = BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: area.w,
            biHeight: -area.h,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..std::mem::zeroed()
        };
        let rows = GetDIBits(memory, bitmap, 0, area.h as u32, bgra.as_mut_ptr().cast(), &mut info, DIB_RGB_COLORS);
        DeleteObject(bitmap);
        DeleteDC(memory);
        ReleaseDC(std::ptr::null_mut(), screen);
        (copied && rows == area.h).then_some(Shot { width, height, bgra })
    }
}
