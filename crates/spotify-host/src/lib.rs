//! The desktop host: a WebView2 window that renders the Spotify web client,
//! driven by the Rust engine in `spotify-core` / `spotify-playback`.
//!
//! The Stock desktop client is the same web client inside a bundled
//! Chromium (CEF, 285 MB) plus a native engine. Hosting the web client in
//! the OS provided WebView2 runtime drops that payload while keeping the
//! interface byte for byte identical.

use std::path::PathBuf;
use std::sync::mpsc;

use tao::event::{Event, StartCause, WindowEvent};
use tao::window::WindowBuilder;
use tao::event_loop::EventLoopBuilder;
use wry::{WebContext, WebView, WebViewBuilder};

pub mod adblock;
pub mod backdrop;
pub mod chrome;
pub mod hotkeys;
pub mod win;

use backdrop::Backdrop;

/// Events the background engine can push into the UI thread.
pub enum HostEvent {
    /// A message from the embedded page.
    FromPage(String),
    /// The engine wants to show something to the user.
    Notify(String),
    /// A hardware media key was pressed.
    MediaKey(hotkeys::MediaKey),
}

/// Where the WebView2 profile (cookies, cache, local storage) lives.
pub fn profile_dir() -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join(".config"))
            .unwrap_or_else(|| PathBuf::from("."))
    };
    base.join("spotify-rs").join("webview")
}

/// Options for the host window.
pub struct HostOptions {
    /// The page to open.
    pub url: String,
    /// Extra JavaScript injected before any page script runs.
    pub bootstrap: String,
    /// Show the developer tools on start.
    pub devtools: bool,
    /// Block advertising and telemetry.
    pub ad_block: bool,
    /// The system backdrop behind the window.
    pub backdrop: Backdrop,
}

impl Default for HostOptions {
    fn default() -> Self {
        Self {
            url: "https://open.spotify.com/".to_string(),
            bootstrap: String::new(),
            devtools: false,
            ad_block: true,
            backdrop: Backdrop::Mica,
        }
    }
}

/// Start the host. Blocks until the window is closed.
pub fn run(options: HostOptions) {
    let profile = profile_dir();
    if let Err(err) = std::fs::create_dir_all(&profile) {
        tracing::warn!("could not create the profile directory: {err}");
    }

    if options.ad_block {
        // Blackhole the advertising hosts at the resolver, so the requests
        // never reach the network at all.
        let existing = std::env::var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS").unwrap_or_default();
        let rules = adblock::host_resolver_rules();
        let arguments = if existing.is_empty() {
            format!("--host-resolver-rules=\"{rules}\"")
        } else {
            format!("{existing} --host-resolver-rules=\"{rules}\"")
        };
        unsafe { std::env::set_var("WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", arguments) };
    }

    let event_loop: tao::event_loop::EventLoop<HostEvent> =
        EventLoopBuilder::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // A persistent context keeps the session and the HTTP cache between
    // launches, which is what makes start-up fast.
    let mut context = WebContext::new(Some(profile.clone()));

    let mut webview: Option<WebView> = None;

    // Global media keys: the hook runs on its own thread and hands the keys
    // over to the event loop.
    {
        let (tx, rx) = mpsc::channel::<hotkeys::MediaKey>();
        hotkeys::set_sender(tx.clone());
        let key_proxy = proxy.clone();
        std::thread::spawn(move || {
            while let Ok(key) = rx.recv() {
                if key_proxy.send_event(HostEvent::MediaKey(key)).is_err() {
                    break;
                }
            }
        });
        hotkeys::start(tx);
    }

    event_loop.run(move |event, target, _control_flow| match event {
        Event::NewEvents(StartCause::Init) => {
            let window = match WindowBuilder::new()
                .with_title("Spotify")
                .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 820.0))
                .with_min_inner_size(tao::dpi::LogicalSize::new(860.0, 560.0))
                .build(target)
            {
                Ok(window) => window,
                Err(err) => {
                    tracing::error!("could not create the window: {err}");
                    std::process::exit(1);
                }
            };

            let mut builder = WebViewBuilder::new_with_web_context(&mut context)
                .with_url(&options.url)
                .with_devtools(options.devtools)
                .with_accept_first_mouse(true)
                .with_transparent(true)
                .with_background_throttling(wry::BackgroundThrottlingPolicy::Disabled);

            // The styling and the ad blocker are injected before the page
            // runs, so nothing flashes unstyled.
            let mut bootstrap = String::new();
            if options.ad_block {
                bootstrap.push_str(&adblock::injected_script());
                bootstrap.push('\n');
            }
            bootstrap.push_str(&chrome::injected_script());
            if !options.bootstrap.is_empty() {
                bootstrap.push('\n');
                bootstrap.push_str(&options.bootstrap);
            }
            builder = builder.with_initialization_script(bootstrap);

            let ipc_proxy = proxy.clone();
            builder = builder.with_ipc_handler(move |request| {
                ipc_proxy
                    .send_event(HostEvent::FromPage(request.body().to_string()))
                    .ok();
            });

            // Give the window the Windows 11 look: rounded corners, a dark
            // caption and the Mica backdrop behind the page.
            {
                use raw_window_handle::{HasWindowHandle, RawWindowHandle};
                if let Ok(handle) = window.window_handle() {
                    if let RawWindowHandle::Win32(win32) = handle.as_raw() {
                        let hwnd = windows::Win32::Foundation::HWND(win32.hwnd.get() as *mut _);
                        win::style_window(hwnd, (18, 18, 18), options.backdrop);
                    }
                }
            }

            match builder.build_as_child(&window) {
                Ok(view) => {
                    tracing::info!("host window ready");
                    // A child web view does not follow the parent window, so
                    // give it the full client area straight away.
                    let size = window.inner_size();
                    let _ = view.set_bounds(wry::Rect {
                        position: tao::dpi::Position::Physical(tao::dpi::PhysicalPosition::new(0, 0)),
                        size: tao::dpi::Size::Physical(size),
                    });
                    webview = Some(view);
                }
                Err(err) => {
                    tracing::error!("could not create the web view: {err}");
                    std::process::exit(1);
                }
            }

            // Keep the window alive for the lifetime of the process.
            std::mem::forget(window);
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(size),
            ..
        } => {
            // Follow the parent window.
            if let Some(view) = &webview {
                let _ = view.set_bounds(wry::Rect {
                    position: tao::dpi::Position::Physical(tao::dpi::PhysicalPosition::new(0, 0)),
                    size: tao::dpi::Size::Physical(size),
                });
            }
        }
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            tracing::info!("closing the host");
            std::process::exit(0);
        }
        Event::UserEvent(HostEvent::FromPage(body)) => {
            tracing::debug!("page message: {body}");
        }
        Event::UserEvent(HostEvent::MediaKey(key)) => {
            tracing::debug!("media key: {key:?}");
            if let Some(view) = &webview {
                let script = hotkeys::dispatch_script(key);
                let _ = view.evaluate_script(&script);
            }
        }
        Event::UserEvent(HostEvent::Notify(text)) => {
            tracing::info!("notify: {text}");
        }
        _ => {}
    });
}

/// A convenience channel pair used by the engine to talk to the UI thread.
pub fn engine_channel() -> (mpsc::Sender<String>, mpsc::Receiver<String>) {
    mpsc::channel()
}
