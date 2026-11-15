//! Global media keys.
//!
//! A browser page never sees the hardware media keys, so a desktop client has
//! to catch them itself. A low level keyboard hook sees them before any other
//! application does, which lets the host drive the player the way a native
//! client would.

use std::sync::mpsc::Sender;

/// The media keys we react to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKey {
    PlayPause,
    Next,
    Previous,
    Stop,
    VolumeUp,
    VolumeDown,
    Mute,
}

const VK_MEDIA_NEXT_TRACK: i32 = 0xB0;
const VK_MEDIA_PREV_TRACK: i32 = 0xB1;
const VK_MEDIA_STOP: i32 = 0xB2;
const VK_MEDIA_PLAY_PAUSE: i32 = 0xB3;
const VK_VOLUME_MUTE: i32 = 0xAD;
const VK_VOLUME_DOWN: i32 = 0xAE;
const VK_VOLUME_UP: i32 = 0xAF;

const WM_KEYDOWN: u32 = 0x0100;
const WM_SYSKEYDOWN: u32 = 0x0101;
const WH_KEYBOARD_LL: windows::Win32::UI::WindowsAndMessaging::WINDOWS_HOOK_ID = windows::Win32::UI::WindowsAndMessaging::WINDOWS_HOOK_ID(13);
const HC_ACTION: i32 = 0;

/// Start the hook on its own thread, forwarding the keys to `sender`.
pub fn start(sender: Sender<MediaKey>) {
    std::thread::spawn(move || unsafe {
        let module = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap_or_default();

        let hook = windows::Win32::UI::WindowsAndMessaging::SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(low_level_proc),
            Some(windows::Win32::Foundation::HINSTANCE(module.0)),
            0,
        );

        if let Ok(hook) = hook {
            // The hook needs a message loop to stay alive.
            let mut message = windows::Win32::UI::WindowsAndMessaging::MSG::default();
            loop {
                let result = windows::Win32::UI::WindowsAndMessaging::GetMessageW(
                    &mut message,
                    None,
                    0,
                    0,
                );
                if result.0 <= 0 {
                    break;
                }
                let _ = windows::Win32::UI::WindowsAndMessaging::TranslateMessage(&message);
                let _ = windows::Win32::UI::WindowsAndMessaging::DispatchMessageW(&message);
            }
            let _ = windows::Win32::UI::WindowsAndMessaging::UnhookWindowsHookEx(hook);
        }

        // Keep the sender alive for the lifetime of the thread.
        drop(sender);
    });
}

thread_local! {
    static SENDER: std::cell::RefCell<Option<Sender<MediaKey>>> = const { std::cell::RefCell::new(None) };
}

/// Install the sender used by the hook callback.
pub fn set_sender(sender: Sender<MediaKey>) {
    SENDER.with(|slot| *slot.borrow_mut() = Some(sender));
}

unsafe extern "system" fn low_level_proc(
    code: i32,
    w_param: windows::Win32::Foundation::WPARAM,
    l_param: windows::Win32::Foundation::LPARAM,
) -> windows::Win32::Foundation::LRESULT {
    use windows::Win32::UI::WindowsAndMessaging::{
        KBDLLHOOKSTRUCT, WM_KEYDOWN, WM_SYSKEYDOWN,
    };

    if code == HC_ACTION as i32 {
        let message = w_param.0 as u32;
        if message == WM_KEYDOWN || message == WM_SYSKEYDOWN {
            let pointer = l_param.0 as *const KBDLLHOOKSTRUCT;
            if !pointer.is_null() {
                let vk = unsafe { (*pointer).vkCode as i32 };
                if let Some(key) = translate(vk) {
                    SENDER.with(|slot| {
                        if let Some(sender) = slot.borrow().as_ref() {
                            let _ = sender.send(key);
                        }
                    });
                }
            }
        }
    }

    unsafe { windows::Win32::UI::WindowsAndMessaging::CallNextHookEx(None, code, w_param, l_param) }
}

fn translate(vk: i32) -> Option<MediaKey> {
    match vk {
        VK_MEDIA_PLAY_PAUSE => Some(MediaKey::PlayPause),
        VK_MEDIA_NEXT_TRACK => Some(MediaKey::Next),
        VK_MEDIA_PREV_TRACK => Some(MediaKey::Previous),
        VK_MEDIA_STOP => Some(MediaKey::Stop),
        VK_VOLUME_UP => Some(MediaKey::VolumeUp),
        VK_VOLUME_DOWN => Some(MediaKey::VolumeDown),
        VK_VOLUME_MUTE => Some(MediaKey::Mute),
        _ => None,
    }
}

/// The script that forwards a media key to the embedded player.
pub fn script_for(key: MediaKey) -> &'static str {
    match key {
        MediaKey::PlayPause => "MediaPlayPause",
        MediaKey::Next => "MediaTrackNext",
        MediaKey::Previous => "MediaTrackPrevious",
        MediaKey::Stop => "MediaStop",
        MediaKey::VolumeUp => "AudioVolumeUp",
        MediaKey::VolumeDown => "AudioVolumeDown",
        MediaKey::Mute => "AudioVolumeMute",
    }
}

/// Dispatch a media key inside the page.
pub fn dispatch_script(key: MediaKey) -> String {
    let name = script_for(key);
    format!(
        r#"(() => {{
  const name = {name};
  for (const type of ['keydown', 'keyup']) {{
    const event = new KeyboardEvent(type, {{
      key: name,
      code: name,
      keyCode: 0,
      which: 0,
      bubbles: true,
      cancelable: true,
      view: window
    }});
    window.dispatchEvent(event);
    document.dispatchEvent(event);
  }}
}})();"#,
        name = serde_json::to_string(name).unwrap_or_else(|_| "\"MediaPlayPause\"".into())
    )
}
