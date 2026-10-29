//! The window through `PrintWindow`, which asks the window and its children
//! to draw into a bitmap. `PW_RENDERFULLCONTENT` includes content composed
//! by DirectComposition, which is how WebView2 panels are drawn.

use windows_sys::Win32::Foundation::{HWND, RECT};
use windows_sys::Win32::Graphics::Gdi::{
    CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
};
use windows_sys::Win32::Storage::Xps::PrintWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::GetClientRect;

use super::bgrx_to_rgba;

/// The client area only, without the title bar and borders.
const PW_CLIENTONLY: u32 = 0x1;
/// Content composed outside GDI, such as WebView2 and Direct3D.
const PW_RENDERFULLCONTENT: u32 = 0x2;

/// The client area of `hwnd` as `(width, height, rgba)`.
pub(super) fn capture(hwnd: isize) -> Result<(u32, u32, Vec<u8>), String> {
    let hwnd = hwnd as HWND;
    // SAFETY: every handle is created and released here, in order, and the
    // pixel buffer is sized for the bitmap GetDIBits fills.
    unsafe {
        let mut rect: RECT = std::mem::zeroed();
        if GetClientRect(hwnd, &mut rect) == 0 {
            return Err("GetClientRect failed".to_string());
        }
        let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
        if width <= 0 || height <= 0 {
            return Err("the window has no client area; is it minimised?".to_string());
        }
        let window_dc = GetDC(hwnd);
        let memory_dc = CreateCompatibleDC(window_dc);
        let bitmap = CreateCompatibleBitmap(window_dc, width, height);
        let previous = SelectObject(memory_dc, bitmap);
        let printed = PrintWindow(hwnd, memory_dc, PW_CLIENTONLY | PW_RENDERFULLCONTENT);

        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = width;
        // Negative: rows top to bottom, as a PNG stores them.
        info.bmiHeader.biHeight = -height;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let rows = GetDIBits(
            memory_dc,
            bitmap,
            0,
            height as u32,
            pixels.as_mut_ptr().cast(),
            &mut info,
            DIB_RGB_COLORS,
        );

        SelectObject(memory_dc, previous);
        DeleteObject(bitmap);
        DeleteDC(memory_dc);
        ReleaseDC(hwnd, window_dc);

        if printed == 0 {
            return Err("PrintWindow failed".to_string());
        }
        if rows != height {
            return Err(format!("GetDIBits read {rows} of {height} rows"));
        }
        bgrx_to_rgba(&mut pixels);
        Ok((width as u32, height as u32, pixels))
    }
}
