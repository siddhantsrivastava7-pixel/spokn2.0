//! Conversation Mode: hands-free dictation loop inside whitelisted
//! macOS chat apps.
//!
//! Architecture:
//!
//!   * [`controller::ControllerCore`] — pure state machine. No audio,
//!     no Tauri, no IO. Fully unit-testable.
//!   * [`app_detector`] — polls `NSWorkspace.frontmostApplication`
//!     and reports supported / unsupported transitions.
//!   * [`vad_loop`] — owns a low-power cpal stream + Silero VAD;
//!     emits speech-start / speech-end edges.
//!   * [`driver`] — Tauri-aware glue. Subscribes to all of the above,
//!     feeds events into the controller, and runs the resulting
//!     [`controller::Action`] list (calls into AudioRecordingManager,
//!     emits Tauri events, drives the countdown timer).
//!
//! v1 is **macOS-only** for the runtime. Settings are cross-platform
//! so a synced install on Windows/Linux still has well-typed values.

pub mod app_detector;
pub mod controller;
pub mod driver;
pub mod vad_loop;

pub use driver::ConversationDriver;
