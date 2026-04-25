//! macOS frontmost-application detector for Conversation Mode.
//!
//! Polls `[NSWorkspace sharedWorkspace] frontmostApplication` on a
//! background thread. Maps the bundle identifier against a small,
//! conservative whitelist of native chat apps. v1 is **macOS-only**;
//! the public type compiles on every platform but always reports
//! `Unsupported` elsewhere so the rest of the controller doesn't need
//! `cfg`-guards.
//!
//! Web-based chat clients (WhatsApp Web, Slack web) are intentionally
//! out of scope for v1 — they require browser URL probing via the
//! Accessibility API which is fragile and merits its own design pass.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Cadence for polling the frontmost app. 500 ms balances responsiveness
/// against CPU cost — at 1 Hz a user could finish an utterance into the
/// wrong app before we noticed; at 10 Hz we'd burn cycles for no gain.
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// Recognised native chat apps. Matching is exact bundle id only —
/// avoid heuristics that could trap the user inside a non-chat app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChatApp {
    Messages,
    WhatsApp,
    Telegram,
    Signal,
    Slack,
    Discord,
}

impl ChatApp {
    pub fn display_name(&self) -> &'static str {
        match self {
            ChatApp::Messages => "Messages",
            ChatApp::WhatsApp => "WhatsApp",
            ChatApp::Telegram => "Telegram",
            ChatApp::Signal => "Signal",
            ChatApp::Slack => "Slack",
            ChatApp::Discord => "Discord",
        }
    }
}

/// Resolve a macOS bundle identifier to a [`ChatApp`] if we recognise
/// it. Pure function — easily unit-testable.
pub fn classify_bundle_id(bundle_id: &str) -> Option<ChatApp> {
    match bundle_id {
        // System Messages app (iMessage / SMS bridge)
        "com.apple.MobileSMS" => Some(ChatApp::Messages),
        // App Store + standalone WhatsApp Desktop both ship as
        // net.whatsapp.WhatsApp; the older Catalyst build was the same.
        "net.whatsapp.WhatsApp" | "WhatsApp" => Some(ChatApp::WhatsApp),
        // Telegram Desktop ships under multiple ids depending on
        // distribution channel (App Store vs. direct).
        "ru.keepcoder.Telegram" | "org.telegram.desktop" => Some(ChatApp::Telegram),
        // Signal Desktop on macOS.
        "org.whispersystems.signal-desktop" => Some(ChatApp::Signal),
        // Slack desktop is the macgap build.
        "com.tinyspeck.slackmacgap" => Some(ChatApp::Slack),
        // Discord PTB / Canary use different ids; ship support for
        // stable only in v1.
        "com.hnc.Discord" => Some(ChatApp::Discord),
        _ => None,
    }
}

/// Snapshot of the current frontmost-app state. Cheap to clone.
#[derive(Debug, Clone)]
pub enum FocusedApp {
    Supported {
        bundle_id: String,
        app: ChatApp,
    },
    Unsupported {
        bundle_id: String,
    },
    Unknown,
}

impl FocusedApp {
    pub fn is_supported(&self) -> bool {
        matches!(self, FocusedApp::Supported { .. })
    }

    pub fn bundle_id(&self) -> Option<&str> {
        match self {
            FocusedApp::Supported { bundle_id, .. }
            | FocusedApp::Unsupported { bundle_id } => Some(bundle_id),
            FocusedApp::Unknown => None,
        }
    }
}

/// Background poller. Calls `on_change(prev, current)` whenever the
/// frontmost app id changes — the consumer (controller) decides what
/// to do (pause / resume / stop).
pub struct AppDetector {
    stop_flag: Arc<AtomicBool>,
    handle: Mutex<Option<JoinHandle<()>>>,
    last: Arc<Mutex<FocusedApp>>,
}

impl AppDetector {
    pub fn new() -> Self {
        Self {
            stop_flag: Arc::new(AtomicBool::new(false)),
            handle: Mutex::new(None),
            last: Arc::new(Mutex::new(FocusedApp::Unknown)),
        }
    }

    pub fn current(&self) -> FocusedApp {
        self.last.lock().unwrap().clone()
    }

    pub fn start<F>(&self, mut on_change: F) -> Result<(), String>
    where
        F: FnMut(&FocusedApp, &FocusedApp) + Send + 'static,
    {
        let mut guard = self.handle.lock().unwrap();
        if guard.is_some() {
            return Ok(()); // already running
        }
        self.stop_flag.store(false, Ordering::Relaxed);
        let stop = self.stop_flag.clone();
        let last = self.last.clone();

        let h = thread::spawn(move || {
            // Prime with an immediate read so consumers don't have to
            // wait POLL_INTERVAL for the first event.
            let initial = read_focused_app();
            {
                let prev = std::mem::replace(&mut *last.lock().unwrap(), initial.clone());
                on_change(&prev, &initial);
            }

            while !stop.load(Ordering::Relaxed) {
                thread::sleep(POLL_INTERVAL);
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let next = read_focused_app();
                let mut last_lock = last.lock().unwrap();
                if !same_app(&last_lock, &next) {
                    let prev = std::mem::replace(&mut *last_lock, next.clone());
                    drop(last_lock);
                    on_change(&prev, &next);
                }
            }
        });
        *guard = Some(h);
        Ok(())
    }

    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.lock().unwrap().take() {
            let _ = h.join();
        }
    }
}

impl Default for AppDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for AppDetector {
    fn drop(&mut self) {
        self.stop();
    }
}

fn same_app(a: &FocusedApp, b: &FocusedApp) -> bool {
    match (a, b) {
        (FocusedApp::Unknown, FocusedApp::Unknown) => true,
        (
            FocusedApp::Supported { bundle_id: a, .. },
            FocusedApp::Supported { bundle_id: b, .. },
        )
        | (
            FocusedApp::Unsupported { bundle_id: a },
            FocusedApp::Unsupported { bundle_id: b },
        ) => a == b,
        _ => false,
    }
}

#[cfg(target_os = "macos")]
fn read_focused_app() -> FocusedApp {
    use cocoa::base::nil;
    use objc::runtime::Object;
    use objc::{class, msg_send, sel, sel_impl};

    unsafe {
        let workspace: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];
        if workspace.is_null() {
            return FocusedApp::Unknown;
        }
        let app: *mut Object = msg_send![workspace, frontmostApplication];
        if app.is_null() {
            return FocusedApp::Unknown;
        }
        let bundle_id: *mut Object = msg_send![app, bundleIdentifier];
        if bundle_id == nil {
            return FocusedApp::Unknown;
        }
        let utf8: *const i8 = msg_send![bundle_id, UTF8String];
        if utf8.is_null() {
            return FocusedApp::Unknown;
        }
        let cstr = std::ffi::CStr::from_ptr(utf8);
        let bid = match cstr.to_str() {
            Ok(s) => s.to_string(),
            Err(_) => return FocusedApp::Unknown,
        };
        match classify_bundle_id(&bid) {
            Some(app) => FocusedApp::Supported { bundle_id: bid, app },
            None => FocusedApp::Unsupported { bundle_id: bid },
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn read_focused_app() -> FocusedApp {
    // v1 is macOS-only; non-macOS builds report nothing supported.
    FocusedApp::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitelist_classifies_messages() {
        assert_eq!(
            classify_bundle_id("com.apple.MobileSMS"),
            Some(ChatApp::Messages)
        );
    }

    #[test]
    fn whitelist_classifies_whatsapp() {
        assert_eq!(
            classify_bundle_id("net.whatsapp.WhatsApp"),
            Some(ChatApp::WhatsApp)
        );
    }

    #[test]
    fn whitelist_classifies_all_six() {
        let ids = [
            ("com.apple.MobileSMS", ChatApp::Messages),
            ("net.whatsapp.WhatsApp", ChatApp::WhatsApp),
            ("ru.keepcoder.Telegram", ChatApp::Telegram),
            ("org.whispersystems.signal-desktop", ChatApp::Signal),
            ("com.tinyspeck.slackmacgap", ChatApp::Slack),
            ("com.hnc.Discord", ChatApp::Discord),
        ];
        for (bid, expected) in ids {
            assert_eq!(classify_bundle_id(bid), Some(expected), "bid: {bid}");
        }
    }

    #[test]
    fn whitelist_rejects_browsers_and_email() {
        // These are exactly the kinds of apps the spec says we must
        // NEVER auto-paste into. Confirm classification returns None.
        for bid in [
            "com.google.Chrome",
            "com.apple.Safari",
            "com.microsoft.Edge",
            "com.apple.mail",
            "com.notion.notion",
            "md.obsidian",
            "com.microsoft.VSCode",
            "com.apple.Terminal",
            "",
            "garbage.unknown.app",
        ] {
            assert!(
                classify_bundle_id(bid).is_none(),
                "should not classify: {bid}"
            );
        }
    }

    #[test]
    fn focused_app_is_supported_helper_works() {
        let a = FocusedApp::Supported {
            bundle_id: "com.apple.MobileSMS".into(),
            app: ChatApp::Messages,
        };
        assert!(a.is_supported());
        assert!(!FocusedApp::Unsupported {
            bundle_id: "x".into()
        }
        .is_supported());
        assert!(!FocusedApp::Unknown.is_supported());
    }
}
