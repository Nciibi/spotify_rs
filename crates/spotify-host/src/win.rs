//! Windows 11 window styling: Mica backdrop, rounded corners, a dark caption
//! that blends into the app, and a transparent web view so the backdrop
//! shows through.

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::DwmSetWindowAttribute;

use crate::backdrop::Backdrop;

/// Use the immersive dark mode for the non-client area.
const DWMWA_USE_IMMERSIVE_DARK_MODE: u32 = 20;
/// Corner preference of the window.
const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
/// Border colour of the window.
const DWMWA_BORDER_COLOR: u32 = 34;
/// Colour of the caption (title bar) area.
const DWMWA_CAPTION_COLOR: u32 = 35;
/// The system backdrop type of the window.
const DWMWA_SYSTEMBACKDROP_TYPE: u32 = 38;

/// `DwmColorDefault`, meaning "let the system decide".
const DWMWA_COLOR_DEFAULT: u32 = 0xFFFF_FFFF;
/// `DwmColorNone`, meaning "no border at all".
const DWMWA_COLOR_NONE: u32 = 0xFFFF_FFFE;

const DWMWCP_ROUND: i32 = 2;

/// Mica.
const DWMSBT_MICA: i32 = 2;
/// Acrylic.
const DWMSBT_ACRYLIC: i32 = 3;

fn set_attribute(hwnd: HWND, attribute: u32, value: u32) {
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            windows::Win32::Graphics::Dwm::DWMWINDOWATTRIBUTE(attribute as i32),
            &value as *const u32 as *const std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        );
    }
}

/// Apply the Windows 11 look to a window.
///
/// `accent` is the RGB value used for the border so it matches the app.
pub fn style_window(hwnd: HWND, accent: (u8, u8, u8), backdrop: Backdrop) {
    // Dark non-client area.
    set_attribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, 1);

    // Rounded corners.
    set_attribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND as u32);

    // Blend the border into the application chrome.
    let (r, g, b) = accent;
    let color = (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b);
    set_attribute(hwnd, DWMWA_BORDER_COLOR, color);

    match backdrop {
        Backdrop::None => {
            set_attribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, 0);
            set_attribute(hwnd, DWMWA_CAPTION_COLOR, color);
        }
        Backdrop::Mica => {
            set_attribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, DWMSBT_MICA as u32);
            // Let the Mica show through the caption.
            set_attribute(hwnd, DWMWA_CAPTION_COLOR, DWMWA_COLOR_DEFAULT);
        }
        Backdrop::Acrylic => {
            set_attribute(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, DWMSBT_ACRYLIC as u32);
            set_attribute(hwnd, DWMWA_CAPTION_COLOR, DWMWA_COLOR_DEFAULT);
        }
    }
}

/// Remove the window border entirely.
pub fn remove_border(hwnd: HWND) {
    set_attribute(hwnd, DWMWA_BORDER_COLOR, DWMWA_COLOR_NONE);
}
