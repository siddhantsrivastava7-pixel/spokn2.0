//! Knock Mode service.
//!
//! Owns the dedicated cpal input stream + the worker thread that runs
//! the [`super::detector::TapDetector`] over its samples. Surfaces a
//! single callback for confirmed double-taps and a calibration mode
//! that reuses the same stream so we never open the mic twice.
//!
//! The implementation is gated on `target_os = "macos"`: on other
//! platforms the public type compiles to no-ops so the rest of the
//! codebase doesn't need conditional plumbing.

use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc, Mutex,
};

use super::calibration::{Calibration, CalibrationOutcome, CalibrationProgress};
use super::detector::{AmpEvent, DetectorOutcome, TapDetector};

/// Callback invoked on confirmed double-taps. Held inside an `Arc` so
/// it can be cheaply cloned into the worker thread.
pub type DoubleTapCallback = Arc<dyn Fn() + Send + Sync + 'static>;

/// Callback for calibration progress + final outcome. The argument
/// reaches `Some(progress)` on every step and the final tick carries
/// `Some(outcome)` once enough taps have been gathered. The service
/// will then auto-stop calibration mode and resume normal listening.
pub type CalibrationCallback =
    Arc<dyn Fn(CalibrationProgress, Option<CalibrationOutcome>) + Send + Sync + 'static>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceState {
    Stopped,
    Listening,
    Calibrating,
}

/// Public, platform-agnostic façade for the knock service.
///
/// The cpal stream is held *outside* `Inner` because it is `!Send` —
/// keeping it on `Inner` would poison the audio-callback closure's
/// `Send` bound. Only data the callback actually needs (thresholds,
/// callbacks, state flags) lives in the `Arc<Inner>` it captures.
pub struct KnockService {
    inner: Arc<Inner>,
    /// Held cpal stream. Dropped on stop. Lives on `KnockService`
    /// (not `Inner`) to keep `Inner: Send + Sync`.
    #[cfg(target_os = "macos")]
    stream: Mutex<Option<cpal::Stream>>,
}

struct Inner {
    callback: DoubleTapCallback,
    /// Calibration callback — `None` when not in a calibration session.
    cal_cb: Mutex<Option<CalibrationCallback>>,
    /// Stored threshold; bits-encoded f32 for atomic updates.
    threshold_bits: AtomicU32,
    /// Stop signal honoured by the worker + audio callback.
    stop_flag: Arc<AtomicBool>,
    state: Mutex<ServiceState>,
}

// Safety: Mutex<Option<cpal::Stream>> is !Send because cpal::Stream is
// !Send. We never move the stream across threads — KnockService is
// constructed on the main thread, the stream is opened from there,
// and dropped from there. Tauri's command runtime always invokes
// commands on the same thread the manager state lives on.
#[cfg(target_os = "macos")]
unsafe impl Send for KnockService {}
#[cfg(target_os = "macos")]
unsafe impl Sync for KnockService {}

impl KnockService {
    pub fn new(callback: DoubleTapCallback, initial_threshold: f32) -> Self {
        let inner = Arc::new(Inner {
            callback,
            cal_cb: Mutex::new(None),
            threshold_bits: AtomicU32::new(initial_threshold.to_bits()),
            stop_flag: Arc::new(AtomicBool::new(false)),
            state: Mutex::new(ServiceState::Stopped),
        });
        Self {
            inner,
            #[cfg(target_os = "macos")]
            stream: Mutex::new(None),
        }
    }

    pub fn state(&self) -> ServiceState {
        *self.inner.state.lock().unwrap()
    }

    pub fn set_threshold(&self, t: f32) {
        let clamped = super::detector::clamp_threshold(t);
        self.inner
            .threshold_bits
            .store(clamped.to_bits(), Ordering::Relaxed);
    }

    /// Start listening. No-op on non-macOS, or if already running.
    #[cfg(target_os = "macos")]
    pub fn start(&self, preferred_device_id: Option<&str>) -> Result<(), String> {
        let mut state = self.inner.state.lock().unwrap();
        if *state != ServiceState::Stopped {
            return Ok(());
        }
        let stream = open_stream(self.inner.clone(), preferred_device_id)?;
        *self.stream.lock().unwrap() = Some(stream);
        *state = ServiceState::Listening;
        log::info!("Knock Mode service started");
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn start(&self, _preferred_device_id: Option<&str>) -> Result<(), String> {
        // Knock Mode is macOS-only in v1; the UI gates the toggle on
        // platform so this should never be called, but stub it safely
        // in case a setting was synced over from a Mac install.
        Err("Knock Mode is only available on macOS".to_string())
    }

    /// Stop listening (or cancel calibration). No-op if already stopped.
    pub fn stop(&self) {
        let mut state = self.inner.state.lock().unwrap();
        if *state == ServiceState::Stopped {
            return;
        }
        self.inner.stop_flag.store(true, Ordering::Relaxed);
        #[cfg(target_os = "macos")]
        {
            // Dropping the stream stops the audio thread.
            *self.stream.lock().unwrap() = None;
        }
        // Reset stop_flag for next start.
        self.inner.stop_flag.store(false, Ordering::Relaxed);
        *self.inner.cal_cb.lock().unwrap() = None;
        *state = ServiceState::Stopped;
        log::info!("Knock Mode service stopped");
    }

    /// Switch the running service into calibration mode. Requires the
    /// service to already be in `Listening`. The `cb` is invoked on
    /// each progress step and once more at completion.
    pub fn start_calibration(&self, cb: CalibrationCallback) -> Result<(), String> {
        let mut state = self.inner.state.lock().unwrap();
        match *state {
            ServiceState::Listening => {
                *self.inner.cal_cb.lock().unwrap() = Some(cb);
                *state = ServiceState::Calibrating;
                log::debug!("Knock Mode entered calibration");
                Ok(())
            }
            ServiceState::Calibrating => {
                Err("Calibration already in progress".to_string())
            }
            ServiceState::Stopped => {
                Err("Knock Mode service is not running".to_string())
            }
        }
    }

    pub fn cancel_calibration(&self) {
        let mut state = self.inner.state.lock().unwrap();
        if *state == ServiceState::Calibrating {
            *self.inner.cal_cb.lock().unwrap() = None;
            *state = ServiceState::Listening;
            log::debug!("Knock Mode calibration cancelled");
        }
    }
}

#[cfg(target_os = "macos")]
fn open_stream(
    inner: Arc<Inner>,
    preferred_device_id: Option<&str>,
) -> Result<cpal::Stream, String> {
    use cpal::traits::{DeviceTrait, StreamTrait};

    let device = pick_device(preferred_device_id)?;
    let device_name = device.name().unwrap_or_else(|_| "<unknown>".into());
    let config = device
        .default_input_config()
        .map_err(|e| format!("default_input_config: {e}"))?;
    let channels = config.channels() as usize;
    let sample_rate = config.sample_rate().0;
    log::info!(
        "Knock Mode listening on '{}' @ {} Hz, {} ch",
        device_name,
        sample_rate,
        channels
    );

    let inner_cb = inner.clone();
    let mut samples_seen: u64 = 0;
    let mut detector =
        TapDetector::new(f32::from_bits(inner.threshold_bits.load(Ordering::Relaxed)));
    let mut calibration: Option<Calibration> = None;

    let err_fn = |e| log::error!("Knock Mode stream error: {e}");

    macro_rules! build_stream {
        ($t:ty) => {{
            let inner_cb = inner_cb.clone();
            device
                .build_input_stream(
                    &config.clone().into(),
                    move |data: &[$t], _: &cpal::InputCallbackInfo| {
                        if inner_cb.stop_flag.load(Ordering::Relaxed) {
                            return;
                        }
                        // Compute mono peak amplitude of this buffer.
                        let mut peak: f32 = 0.0;
                        if channels == 1 {
                            for &s in data {
                                let v: f32 = cpal::Sample::to_sample::<f32>(s).abs();
                                if v > peak {
                                    peak = v;
                                }
                            }
                        } else {
                            for frame in data.chunks_exact(channels) {
                                let mut sum: f32 = 0.0;
                                for &s in frame {
                                    sum += cpal::Sample::to_sample::<f32>(s);
                                }
                                let avg = (sum / channels as f32).abs();
                                if avg > peak {
                                    peak = avg;
                                }
                            }
                        }
                        // Update the time cursor by buffer duration so
                        // the detector sees a monotonic clock decoupled
                        // from wall time (avoids drift / sleep stalls).
                        samples_seen += data.len() as u64 / channels.max(1) as u64;
                        let t_ms = (samples_seen * 1000) / sample_rate.max(1) as u64;

                        // Refresh threshold each tick — cheap atomic
                        // load, lets the user nudge sensitivity at
                        // runtime without restarting the stream.
                        let cur = f32::from_bits(
                            inner_cb.threshold_bits.load(Ordering::Relaxed),
                        );
                        if (cur - detector.threshold()).abs() > f32::EPSILON {
                            detector.set_threshold(cur);
                        }

                        let ev = AmpEvent { t_ms, peak };

                        // Calibration path: feed Calibration first;
                        // when it announces completion, snapshot the
                        // outcome, store it as the new threshold, fire
                        // the cal callback and exit calibration mode.
                        let in_cal = inner_cb.cal_cb.lock().unwrap().is_some();
                        if in_cal {
                            if calibration.is_none() {
                                calibration = Some(Calibration::new());
                            }
                            if let Some(cal) = calibration.as_mut() {
                                if let Some(progress) = cal.feed(ev) {
                                    if let Some(cb) =
                                        inner_cb.cal_cb.lock().unwrap().as_ref()
                                    {
                                        cb(progress, None);
                                    }
                                    if progress.double_taps_collected
                                        >= progress.double_taps_required
                                    {
                                        // Take ownership to compute the outcome.
                                        let owned = calibration.take().unwrap();
                                        if let Some(out) = owned.finish() {
                                            log::info!(
                                                "Knock calibration complete: noise={:.4} \
                                                 avg_tap={:.4} threshold={:.4}",
                                                out.noise_floor,
                                                out.avg_tap_peak,
                                                out.threshold
                                            );
                                            inner_cb
                                                .threshold_bits
                                                .store(out.threshold.to_bits(), Ordering::Relaxed);
                                            detector.set_threshold(out.threshold);
                                            if let Some(cb) =
                                                inner_cb.cal_cb.lock().unwrap().as_ref()
                                            {
                                                cb(progress, Some(out));
                                            }
                                            // Exit calibration: clear cb, restore Listening.
                                            *inner_cb.cal_cb.lock().unwrap() = None;
                                            *inner_cb.state.lock().unwrap() =
                                                ServiceState::Listening;
                                        }
                                    }
                                }
                            }
                            return;
                        } else if calibration.is_some() {
                            // Cancelled mid-calibration; drop state.
                            calibration = None;
                        }

                        // Normal detection path.
                        match detector.tick(ev) {
                            DetectorOutcome::DoubleTap => {
                                log::debug!("Knock Mode: double-tap @ {} ms", t_ms);
                                let cb = inner_cb.callback.clone();
                                // Fire on a worker thread — the audio
                                // callback must NEVER block on app
                                // logic (recording start, IPC, etc).
                                std::thread::spawn(move || (cb)());
                            }
                            DetectorOutcome::Rejected(reason) => {
                                log::trace!(
                                    "Knock Mode: rejected ({:?}) peak={:.4} t={}",
                                    reason,
                                    peak,
                                    t_ms
                                );
                                let _ = reason; // silence unused-binding when log filtered
                            }
                            _ => {}
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("build_input_stream: {e}"))
        }};
    }

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build_stream!(f32)?,
        cpal::SampleFormat::I16 => build_stream!(i16)?,
        cpal::SampleFormat::I32 => build_stream!(i32)?,
        cpal::SampleFormat::U16 => build_stream!(u16)?,
        other => {
            return Err(format!(
                "Knock Mode: unsupported sample format {:?}",
                other
            ))
        }
    };

    stream
        .play()
        .map_err(|e| format!("stream.play: {e}"))?;

    Ok(stream)
}

#[cfg(target_os = "macos")]
fn pick_device(preferred_id: Option<&str>) -> Result<cpal::Device, String> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();

    // 1. Honour explicit user override, by name match.
    if let Some(want) = preferred_id {
        if let Ok(devs) = host.input_devices() {
            for d in devs {
                if d.name().map(|n| n == want).unwrap_or(false) {
                    return Ok(d);
                }
            }
        }
    }

    // 2. Prefer built-in mic — heuristic on common macOS device names.
    if let Ok(devs) = host.input_devices() {
        for d in devs {
            if let Ok(name) = d.name() {
                let lower = name.to_lowercase();
                if lower.contains("macbook")
                    || lower.contains("built-in")
                    || lower.contains("internal")
                {
                    return Ok(d);
                }
            }
        }
    }

    // 3. Fall back to system default.
    host.default_input_device()
        .ok_or_else(|| "No default input device".to_string())
}
