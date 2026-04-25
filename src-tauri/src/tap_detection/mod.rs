//! Knock Mode: hands-free start/stop via a double-tap near the
//! trackpad.
//!
//! Three pieces:
//!   - [`detector`] — pure tap/double-tap state machine, fully
//!     unit-testable, no audio I/O.
//!   - [`calibration`] — per-device threshold derivation from three
//!     deliberate double-taps + ambient noise.
//!   - [`service`] — owns the cpal stream and bridges samples →
//!     detector → callback. macOS-only audio engine; no-op stub on
//!     other platforms (the public types compile everywhere so the
//!     rest of the app doesn't need cfg-guards).
//!
//! Design principle: the audio callback must never block on app
//! logic. When a double-tap is confirmed, the callback ships work to
//! a one-shot worker thread that calls into the existing
//! `AudioRecordingManager`. There is exactly one recording path —
//! Knock Mode is a new *trigger*, not a new pipeline.

pub mod calibration;
pub mod detector;
pub mod service;

pub use calibration::{CalibrationOutcome, CalibrationProgress};
pub use service::{KnockService, ServiceState};
