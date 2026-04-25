//! Pure tap-pattern state machine.
//!
//! Consumes a stream of `(timestamp_ms, peak_amplitude)` measurements and
//! emits exactly one [`DetectorOutcome::DoubleTap`] when a clean
//! double-tap is observed. Nothing in this file talks to audio I/O —
//! this is plain math + a state machine, so every behaviour is unit-
//! testable on synthetic events.
//!
//! # Algorithm
//! Conservative by design. False positives (phantom recordings) are
//! worse than false negatives, so every threshold leans strict.
//!
//! 1. **Trigger**: a sample with `peak > threshold` arms a candidate tap
//!    at time `t`.
//! 2. **Tap-shape filter**: the candidate is only confirmed once the
//!    next ≤ `MAX_TAP_DURATION_MS` of samples *all* fall back below the
//!    threshold. Sustained loud audio (voice, music) fails this step.
//! 3. **Tap debounce**: another tap within `TAP_DEBOUNCE_MS` after a
//!    confirmed tap is ignored — covers edge ringing in the mic.
//! 4. **Double-tap window**: a second confirmed tap arriving in
//!    `[DOUBLE_TAP_MIN_GAP_MS, DOUBLE_TAP_MAX_GAP_MS]` after the first
//!    fires `DoubleTap`.
//! 5. **Typing rejection**: 3+ taps within `TYPING_REJECT_WINDOW_MS`
//!    aborts the candidate — typing on the chassis produces rapid
//!    repeats.
//! 6. **Cooldown**: after firing, ignore *everything* for
//!    `COOLDOWN_MS` so the user's natural follow-through (palm lift,
//!    chair creak) doesn't immediately re-trigger.

/// Hard caps; chosen empirically. Knobs the user shouldn't need to know
/// about. The threshold is the only knob exposed via calibration.
pub const MAX_TAP_DURATION_MS: u64 = 30;
pub const TAP_DEBOUNCE_MS: u64 = 80;
pub const DOUBLE_TAP_MIN_GAP_MS: u64 = 60;
pub const DOUBLE_TAP_MAX_GAP_MS: u64 = 500;
pub const TYPING_REJECT_WINDOW_MS: u64 = 700;
pub const TYPING_REJECT_COUNT: usize = 3;
pub const COOLDOWN_MS: u64 = 800;
pub const SUSTAINED_VOICE_MS: u64 = 100;
/// Confirmation delay after a candidate double-tap. We hold-fire for
/// this long to catch typing patterns (a third tap landing during this
/// window converts the candidate into a `TypingPattern` rejection).
/// 200 ms is barely perceptible to the user but reliably catches typing
/// cadence (~80–120 ms inter-key on average).
pub const DOUBLE_TAP_CONFIRM_DELAY_MS: u64 = 200;

/// Safe clamp range for the calibrated threshold. Anything outside this
/// window is either trivially noisy (will trigger constantly) or so
/// strict it would require an actual hammer.
pub const THRESHOLD_MIN: f32 = 0.05;
pub const THRESHOLD_MAX: f32 = 0.5;

/// One amplitude measurement. The producer (audio worker) computes this
/// per cpal buffer; the detector treats them as opaque.
#[derive(Debug, Clone, Copy)]
pub struct AmpEvent {
    pub t_ms: u64,
    pub peak: f32,
}

/// Reason a candidate tap was rejected — surfaced via debug logs only.
/// Variants are intentionally fine-grained so users running with
/// `--debug` can tell *why* their attempts aren't registering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    /// Sustained loud audio — looks like voice or a held note.
    SustainedEnergy,
    /// 3+ taps in a short window — typing or drumming.
    TypingPattern,
    /// Inside the debounce window after a previous tap.
    Debounced,
    /// Inside the post-trigger cooldown.
    Cooldown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectorOutcome {
    /// Nothing actionable this tick.
    Idle,
    /// First clean tap of a potential double-tap landed; UI may want to
    /// show a subtle hint but should NOT start recording yet.
    SingleTapArmed,
    /// Confirmed double-tap. The service should fire its callback.
    DoubleTap,
    /// Diagnostic only — never affects state observably; carried for
    /// logs.
    Rejected(RejectReason),
}

/// Tracks the outline of a tap as we receive samples. A "candidate"
/// becomes a confirmed tap once the energy returns to baseline within
/// `MAX_TAP_DURATION_MS` of arming.
#[derive(Debug, Clone, Copy)]
struct InflightTap {
    started_t_ms: u64,
}

/// State of the detector across calls. `tick` mutates it in place.
#[derive(Debug, Clone)]
pub struct TapDetector {
    threshold: f32,
    /// Tap currently being shaped (energy still above threshold).
    inflight: Option<InflightTap>,
    /// Confirmed taps, used for the double-tap match and typing reject.
    /// Bounded length — we trim anything older than the typing window.
    recent_confirmed: Vec<u64>,
    /// Last debounce barrier — no new tap can arm before this time.
    debounce_until_ms: u64,
    /// Post-trigger cooldown; suppresses everything until this time.
    cooldown_until_ms: u64,
    /// Candidate double-tap awaiting confirmation. We hold-fire to
    /// catch typing patterns: if a 3rd tap arrives before this time
    /// the candidate becomes a `TypingPattern` rejection instead.
    /// `None` when no candidate is pending.
    pending_fire_at_ms: Option<u64>,
}

impl TapDetector {
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold: clamp_threshold(threshold),
            inflight: None,
            recent_confirmed: Vec::new(),
            debounce_until_ms: 0,
            cooldown_until_ms: 0,
            pending_fire_at_ms: None,
        }
    }

    pub fn threshold(&self) -> f32 {
        self.threshold
    }

    pub fn set_threshold(&mut self, t: f32) {
        self.threshold = clamp_threshold(t);
    }

    /// Process one amplitude event and return the resulting outcome.
    /// `Idle` / `SingleTapArmed` are non-actionable; only `DoubleTap`
    /// should drive recording start/stop.
    pub fn tick(&mut self, ev: AmpEvent) -> DetectorOutcome {
        // 1. Cooldown is the highest-priority suppressor — it wins over
        //    anything else. Lets the confirmed double-tap have its
        //    follow-through silence without re-arming.
        if ev.t_ms < self.cooldown_until_ms {
            return DetectorOutcome::Rejected(RejectReason::Cooldown);
        }

        // 2. Pending-fire window: a candidate double-tap is being
        //    held to see if a 3rd tap follows (typing). When the grace
        //    window elapses with no further tap, fire it.
        if let Some(fire_at) = self.pending_fire_at_ms {
            if ev.t_ms >= fire_at {
                self.pending_fire_at_ms = None;
                self.recent_confirmed.clear();
                self.cooldown_until_ms = ev.t_ms + COOLDOWN_MS;
                return DetectorOutcome::DoubleTap;
            }
        }

        let above = ev.peak >= self.threshold;

        if let Some(inflight) = self.inflight {
            let elapsed = ev.t_ms.saturating_sub(inflight.started_t_ms);

            if above {
                // Still loud after MAX_TAP_DURATION_MS → not a tap, it's
                // sustained energy (voice, fan turn-on, etc).
                if elapsed >= SUSTAINED_VOICE_MS {
                    self.inflight = None;
                    return DetectorOutcome::Rejected(
                        RejectReason::SustainedEnergy,
                    );
                }
                return DetectorOutcome::Idle;
            }

            // Below threshold: candidate has shaped successfully if it
            // collapsed within the tap-shape window.
            if elapsed > MAX_TAP_DURATION_MS {
                // Energy dropped, but the loud span was too long for a
                // tap. Treat as voice and discard.
                self.inflight = None;
                return DetectorOutcome::Rejected(RejectReason::SustainedEnergy);
            }

            // CONFIRMED tap.
            self.inflight = None;
            self.debounce_until_ms = ev.t_ms + TAP_DEBOUNCE_MS;
            self.recent_confirmed.push(ev.t_ms);
            self.trim_recent(ev.t_ms);

            // If we already had a candidate double-tap pending, this
            // new tap is a 3rd hit → typing pattern.
            if self.pending_fire_at_ms.is_some() {
                self.pending_fire_at_ms = None;
                self.recent_confirmed.clear();
                self.cooldown_until_ms = ev.t_ms + 300;
                return DetectorOutcome::Rejected(RejectReason::TypingPattern);
            }

            // Typing/drumming check — 3+ taps in the window means the
            // user isn't deliberately knocking, they're typing or
            // shifting around.
            if self.recent_confirmed.len() >= TYPING_REJECT_COUNT {
                self.recent_confirmed.clear();
                self.cooldown_until_ms = ev.t_ms + 300;
                return DetectorOutcome::Rejected(RejectReason::TypingPattern);
            }

            // Double-tap window check.
            if self.recent_confirmed.len() == 2 {
                let gap = self.recent_confirmed[1] - self.recent_confirmed[0];
                if (DOUBLE_TAP_MIN_GAP_MS..=DOUBLE_TAP_MAX_GAP_MS).contains(&gap)
                {
                    // Don't fire yet — hold for the confirmation window
                    // so a 3rd-tap typing pattern can preempt this.
                    self.pending_fire_at_ms =
                        Some(ev.t_ms + DOUBLE_TAP_CONFIRM_DELAY_MS);
                    return DetectorOutcome::SingleTapArmed;
                }
                // Gap was wrong — drop the older tap, treat the newer
                // one as the start of a fresh attempt.
                self.recent_confirmed.remove(0);
            }

            return DetectorOutcome::SingleTapArmed;
        }

        // Not in-flight.
        if above {
            if ev.t_ms < self.debounce_until_ms {
                return DetectorOutcome::Rejected(RejectReason::Debounced);
            }
            self.inflight = Some(InflightTap {
                started_t_ms: ev.t_ms,
            });
        }
        DetectorOutcome::Idle
    }

    /// Drop any state older than the typing-reject window so memory
    /// stays bounded over long sessions.
    fn trim_recent(&mut self, now_ms: u64) {
        let cutoff = now_ms.saturating_sub(TYPING_REJECT_WINDOW_MS);
        self.recent_confirmed.retain(|t| *t >= cutoff);
    }
}

pub fn clamp_threshold(t: f32) -> f32 {
    t.clamp(THRESHOLD_MIN, THRESHOLD_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: feed a contiguous "loud span" (above threshold) followed
    /// by a "quiet span" (below threshold). Returns outcomes in order.
    fn feed_span(
        det: &mut TapDetector,
        start_ms: u64,
        loud_ms: u64,
        loud_peak: f32,
        quiet_ms: u64,
    ) -> Vec<DetectorOutcome> {
        let mut out = Vec::new();
        // Sample every 5 ms — matches real-world ~16ms buffer cadence
        // closely enough.
        let step = 5u64;
        let mut t = start_ms;
        let loud_end = start_ms + loud_ms;
        while t < loud_end {
            out.push(det.tick(AmpEvent {
                t_ms: t,
                peak: loud_peak,
            }));
            t += step;
        }
        let quiet_end = loud_end + quiet_ms;
        while t < quiet_end {
            out.push(det.tick(AmpEvent { t_ms: t, peak: 0.01 }));
            t += step;
        }
        out
    }

    fn double_tapped(events: &[DetectorOutcome]) -> bool {
        events.iter().any(|e| matches!(e, DetectorOutcome::DoubleTap))
    }

    #[test]
    fn double_tap_300ms_apart_confirms() {
        let mut det = TapDetector::new(0.18);
        let mut all = feed_span(&mut det, 0, 15, 0.5, 50);
        // Tap 2, plus enough quiet to clear the 200ms confirmation
        // grace window.
        all.extend(feed_span(&mut det, 300, 15, 0.5, 250));
        assert!(double_tapped(&all), "events: {:?}", all);
    }

    #[test]
    fn taps_700ms_apart_not_confirmed() {
        let mut det = TapDetector::new(0.18);
        let mut all = feed_span(&mut det, 0, 15, 0.5, 50);
        all.extend(feed_span(&mut det, 700, 15, 0.5, 250));
        assert!(!double_tapped(&all), "events: {:?}", all);
    }

    #[test]
    fn rapid_typing_pattern_rejected() {
        let mut det = TapDetector::new(0.18);
        let mut all = Vec::new();
        // 4 taps spaced 100ms apart — typical typing cadence. Each
        // tap shape is short and the third tap arrives well inside
        // the 200ms confirmation grace window of the second.
        for i in 0..4 {
            all.extend(feed_span(&mut det, i * 100, 10, 0.5, 30));
        }
        // Drain past the (hypothetical) pending grace window.
        all.extend(feed_span(&mut det, 600, 0, 0.0, 250));
        assert!(!double_tapped(&all), "events: {:?}", all);
        assert!(
            all.iter().any(|e| matches!(
                e,
                DetectorOutcome::Rejected(RejectReason::TypingPattern)
            )),
            "expected TypingPattern reject, got: {:?}",
            all
        );
    }

    #[test]
    fn sustained_voice_energy_rejected() {
        let mut det = TapDetector::new(0.18);
        // 300ms of continuous loud audio — voice, fan, music.
        let all = feed_span(&mut det, 0, 300, 0.5, 50);
        assert!(!double_tapped(&all), "events: {:?}", all);
        assert!(
            all.iter().any(|e| matches!(
                e,
                DetectorOutcome::Rejected(RejectReason::SustainedEnergy)
            )),
            "expected SustainedEnergy reject, got: {:?}",
            all
        );
    }

    #[test]
    fn cooldown_blocks_immediate_retrigger() {
        let mut det = TapDetector::new(0.18);
        // First confirmed double-tap (with grace window pad).
        let mut all = feed_span(&mut det, 0, 15, 0.5, 50);
        all.extend(feed_span(&mut det, 300, 15, 0.5, 250));
        assert!(double_tapped(&all));
        // Immediately attempt another double-tap inside the cooldown
        // (which begins at the moment the first DoubleTap fires, i.e.
        // ~315 + 200 = ~515 ms, and lasts 800 ms).
        let next_window_start = 600;
        let mut second = feed_span(&mut det, next_window_start, 15, 0.5, 50);
        second.extend(feed_span(&mut det, next_window_start + 250, 15, 0.5, 250));
        assert!(
            !double_tapped(&second),
            "cooldown should suppress, got: {:?}",
            second
        );
    }

    #[test]
    fn peak_below_threshold_ignored() {
        let mut det = TapDetector::new(0.18);
        // Two soft taps that never breach the threshold.
        let mut all = feed_span(&mut det, 0, 15, 0.05, 50);
        all.extend(feed_span(&mut det, 300, 15, 0.05, 50));
        assert!(!double_tapped(&all), "events: {:?}", all);
        assert!(all.iter().all(|e| matches!(e, DetectorOutcome::Idle)));
    }

    #[test]
    fn calibration_threshold_clamped_to_safe_range() {
        assert_eq!(clamp_threshold(0.0), THRESHOLD_MIN);
        assert_eq!(clamp_threshold(2.0), THRESHOLD_MAX);
        let safe = clamp_threshold(0.20);
        assert!(safe > THRESHOLD_MIN && safe < THRESHOLD_MAX);
    }

    #[test]
    fn double_tap_after_cooldown_works() {
        // Regression: confirms the detector self-resets after cooldown.
        let mut det = TapDetector::new(0.18);
        let mut first = feed_span(&mut det, 0, 15, 0.5, 50);
        first.extend(feed_span(&mut det, 300, 15, 0.5, 250));
        assert!(double_tapped(&first));
        // Wait past cooldown (>800ms after second confirmed tap +
        // confirmation grace).
        let resume = 300 + 15 + 200 + 1000;
        let mut second = feed_span(&mut det, resume, 15, 0.5, 50);
        second.extend(feed_span(&mut det, resume + 300, 15, 0.5, 250));
        assert!(double_tapped(&second), "events: {:?}", second);
    }
}
