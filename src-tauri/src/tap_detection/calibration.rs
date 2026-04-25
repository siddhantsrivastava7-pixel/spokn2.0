//! Calibration: collect three deliberate double-taps from the user and
//! distil them into a per-device threshold.
//!
//! Independent from the live detector — the calibration session
//! consumes the same `AmpEvent` stream but tracks peaks and noise
//! floor separately, then emits a final `CalibrationOutcome` that
//! becomes the persisted `knock_threshold`.
//!
//! Formula (matches the spec):
//!   threshold = max(noise_floor * 6, average_tap_peak * 0.45)
//!   threshold = clamp(threshold, MIN, MAX)
//!
//! The 6× noise multiplier guarantees a comfortable margin above
//! ambient room noise; the 0.45× tap multiplier guarantees the
//! threshold stays below the user's own taps even if they tap softer
//! during real use than during calibration.

use super::detector::{
    clamp_threshold, AmpEvent, DetectorOutcome, TapDetector,
};

/// How many double-taps the user must complete before calibration is
/// allowed to finish. Three is the sweet spot — one is noisy, five is
/// tedious.
pub const REQUIRED_DOUBLE_TAPS: usize = 3;

/// Minimum number of "quiet" amplitude samples we need before we trust
/// the noise-floor estimate. Roughly 1 second at 16 kHz with 256-sample
/// buffers.
const MIN_NOISE_SAMPLES: usize = 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationStage {
    /// Waiting for the next double-tap.
    Collecting,
    /// All required samples gathered; threshold ready to apply.
    Done,
}

#[derive(Debug, Clone, Copy)]
pub struct CalibrationProgress {
    pub stage: CalibrationStage,
    pub double_taps_collected: usize,
    pub double_taps_required: usize,
}

/// Final result of a calibration session.
#[derive(Debug, Clone, Copy)]
pub struct CalibrationOutcome {
    pub threshold: f32,
    pub noise_floor: f32,
    pub avg_tap_peak: f32,
}

/// Stateful calibration session. The owning service feeds it the same
/// `AmpEvent` stream the detector sees; once `progress().stage ==
/// Done`, call `finish()` to obtain the threshold.
pub struct Calibration {
    /// Inner detector — it does the heavy lifting of confirming
    /// double-taps. We just shadow its `threshold` to start LOW so
    /// calibration captures even soft taps.
    detector: TapDetector,
    noise_samples: Vec<f32>,
    tap_peaks: Vec<f32>,
    /// Peak observed in the current in-flight tap (for averaging).
    inflight_peak: f32,
    double_taps: usize,
    /// Snapshot of "loud" floor — used to gate which samples count as
    /// noise vs. tap. We update it as we go.
    running_loudest_quiet: f32,
}

impl Calibration {
    pub fn new() -> Self {
        Self {
            // Start at the lowest safe threshold so we don't miss soft
            // calibration taps. The final threshold will be much higher.
            detector: TapDetector::new(0.05),
            noise_samples: Vec::new(),
            tap_peaks: Vec::new(),
            inflight_peak: 0.0,
            double_taps: 0,
            running_loudest_quiet: 0.0,
        }
    }

    pub fn progress(&self) -> CalibrationProgress {
        CalibrationProgress {
            stage: if self.double_taps >= REQUIRED_DOUBLE_TAPS {
                CalibrationStage::Done
            } else {
                CalibrationStage::Collecting
            },
            double_taps_collected: self.double_taps,
            double_taps_required: REQUIRED_DOUBLE_TAPS,
        }
    }

    /// Feed one amplitude sample. Returns `Some(progress)` only when
    /// the count of confirmed double-taps actually advanced (so the
    /// caller can emit a UI event); `None` otherwise.
    pub fn feed(&mut self, ev: AmpEvent) -> Option<CalibrationProgress> {
        // Track per-tap peaks: while above threshold, update the
        // running peak; on dropping back below, lock it in.
        if ev.peak >= self.detector.threshold() {
            if ev.peak > self.inflight_peak {
                self.inflight_peak = ev.peak;
            }
        } else if self.inflight_peak > 0.0 {
            self.tap_peaks.push(self.inflight_peak);
            self.inflight_peak = 0.0;
        } else {
            // Genuinely quiet — feeds the noise floor.
            self.noise_samples.push(ev.peak);
            if ev.peak > self.running_loudest_quiet {
                self.running_loudest_quiet = ev.peak;
            }
        }

        let outcome = self.detector.tick(ev);
        if matches!(outcome, DetectorOutcome::DoubleTap) {
            self.double_taps += 1;
            return Some(self.progress());
        }
        None
    }

    /// Compute the final threshold. Returns `None` if we don't yet
    /// have enough samples to trust the result.
    pub fn finish(self) -> Option<CalibrationOutcome> {
        if self.double_taps < REQUIRED_DOUBLE_TAPS {
            return None;
        }
        if self.noise_samples.len() < MIN_NOISE_SAMPLES
            || self.tap_peaks.is_empty()
        {
            return None;
        }
        let noise_floor = mean(&self.noise_samples);
        let avg_tap_peak = mean(&self.tap_peaks);
        let raw = (noise_floor * 6.0).max(avg_tap_peak * 0.45);
        let threshold = clamp_threshold(raw);
        Some(CalibrationOutcome {
            threshold,
            noise_floor,
            avg_tap_peak,
        })
    }
}

impl Default for Calibration {
    fn default() -> Self {
        Self::new()
    }
}

fn mean(xs: &[f32]) -> f32 {
    if xs.is_empty() {
        return 0.0;
    }
    let sum: f32 = xs.iter().copied().sum();
    sum / (xs.len() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive a synthetic session with three clean double-taps and a
    /// long quiet stretch so the noise floor is well-estimated.
    #[test]
    fn three_double_taps_produces_threshold_in_range() {
        let mut cal = Calibration::new();
        // Generous quiet padding to satisfy MIN_NOISE_SAMPLES.
        let mut t = 0u64;
        for _ in 0..120 {
            let _ = cal.feed(AmpEvent { t_ms: t, peak: 0.005 });
            t += 5;
        }
        // Three double-taps spaced well apart.
        let pairs = [(t, t + 250), (t + 1500, t + 1750), (t + 3500, t + 3750)];
        for (a, b) in pairs {
            push_tap(&mut cal, a, 0.6);
            // Long trailing quiet on tap 2 so the detector's
            // confirmation grace window elapses (real cpal streams
            // emit continuous samples so this isn't an issue at
            // runtime; tests have to simulate the cadence).
            push_long_tap(&mut cal, b, 0.6);
        }
        let progress = cal.progress();
        assert_eq!(
            progress.double_taps_collected, REQUIRED_DOUBLE_TAPS,
            "got: {:?}",
            progress
        );
        let out = cal.finish().expect("calibration should finish");
        assert!(out.threshold >= super::super::detector::THRESHOLD_MIN);
        assert!(out.threshold <= super::super::detector::THRESHOLD_MAX);
        // Conservative formula should keep the threshold meaningfully
        // below the tap peak (so real taps clear it) but well above
        // ambient.
        assert!(out.threshold < 0.6, "threshold too high: {:?}", out);
        assert!(out.threshold > 0.005, "threshold too low: {:?}", out);
    }

    #[test]
    fn finish_rejects_when_not_enough_double_taps() {
        let mut cal = Calibration::new();
        let mut t = 0u64;
        for _ in 0..120 {
            let _ = cal.feed(AmpEvent { t_ms: t, peak: 0.005 });
            t += 5;
        }
        // Only one double-tap.
        push_tap(&mut cal, t, 0.6);
        push_tap(&mut cal, t + 250, 0.6);
        assert!(cal.finish().is_none());
    }

    fn push_tap(cal: &mut Calibration, start_ms: u64, peak: f32) {
        // 15ms loud, 50ms quiet — same shape the detector tests use.
        let mut t = start_ms;
        let loud_end = start_ms + 15;
        while t < loud_end {
            let _ = cal.feed(AmpEvent { t_ms: t, peak });
            t += 5;
        }
        let quiet_end = loud_end + 50;
        while t < quiet_end {
            let _ = cal.feed(AmpEvent { t_ms: t, peak: 0.005 });
            t += 5;
        }
    }

    fn push_long_tap(cal: &mut Calibration, start_ms: u64, peak: f32) {
        // Same as push_tap but with extended trailing quiet (250 ms)
        // so the detector's pending DoubleTap fires during the gap.
        let mut t = start_ms;
        let loud_end = start_ms + 15;
        while t < loud_end {
            let _ = cal.feed(AmpEvent { t_ms: t, peak });
            t += 5;
        }
        let quiet_end = loud_end + 250;
        while t < quiet_end {
            let _ = cal.feed(AmpEvent { t_ms: t, peak: 0.005 });
            t += 5;
        }
    }

    #[test]
    fn finish_rejects_when_not_enough_double_taps_after_one() {
        let mut cal = Calibration::new();
        let mut t = 0u64;
        for _ in 0..120 {
            let _ = cal.feed(AmpEvent { t_ms: t, peak: 0.005 });
            t += 5;
        }
        push_tap(&mut cal, t, 0.6);
        push_long_tap(&mut cal, t + 250, 0.6);
        assert_eq!(cal.progress().double_taps_collected, 1);
        assert!(cal.finish().is_none());
    }
}
