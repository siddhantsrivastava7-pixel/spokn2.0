# Knock Mode

Hands-free start/stop for dictation: double-tap your MacBook near the
trackpad to start recording, double-tap again to stop.

This module is the listener and detector. It does **not** implement a
second recording path — the confirmed-double-tap callback delegates to
the existing `AudioRecordingManager` + `TranscribeAction` flow used by
the global hotkey.

## Module layout

| File | Role |
| --- | --- |
| `detector.rs` | Pure state machine: `(t_ms, peak_amp)` events → `DoubleTap` outcomes. No audio I/O. Unit-tested. |
| `calibration.rs` | Stateful session that listens for three deliberate double-taps + ambient samples, then derives a per-device threshold. |
| `service.rs` | Owns the cpal input stream + audio callback. macOS-only impl; no-op stub on other platforms. |
| `mod.rs` | Public façade. |

## Algorithm

For every cpal input buffer (~16 ms at 16 kHz):

1. Compute the buffer's peak absolute amplitude (mono mix-down if the
   device is multi-channel).
2. Compare against the calibrated threshold.
3. Feed `(timestamp_ms, peak)` into the detector state machine.

The detector enforces these rules — every threshold is intentionally
strict because a phantom recording is worse than a missed knock:

| Rule | Constant | Behaviour |
| --- | --- | --- |
| Tap shape | `MAX_TAP_DURATION_MS = 30` | Energy must drop back below threshold within 30 ms or it's not a tap. |
| Tap debounce | `TAP_DEBOUNCE_MS = 80` | Mic ringing right after a tap is ignored. |
| Double-tap window | 60–500 ms | Second tap arriving outside this window restarts the search. |
| Typing reject | 3+ taps within 700 ms | Resets state. Typing on the chassis hits this guard. |
| Voice reject | sustained > 100 ms | Voice / fan / music never produces a confirmation. |
| Cooldown | 800 ms after fire | Suppresses the natural follow-through (palm lift, chair). |

Confirmation fires the user callback on a worker thread; the cpal
audio thread is never blocked.

## Calibration

User taps three deliberate double-taps. The session collects:

- Per-tap peak amplitude (for an average true-positive level).
- Ambient samples (for a noise-floor estimate, ≥ 60 samples ≈ 1 s).

Final threshold:

```text
threshold = max(noise_floor * 6, avg_tap_peak * 0.45)
threshold = clamp(threshold, 0.05, 0.5)
```

The 6× noise multiplier guarantees a comfortable margin over ambient
hiss. The 0.45× tap multiplier guarantees the threshold stays
comfortably below the user's own taps even if they tap softer at
runtime than during calibration. The `[0.05, 0.5]` clamp prevents
either trivially-noisy or unreachably-strict thresholds.

## Settings

Persisted in `AppSettings`:

- `knock_mode_enabled: bool` — default `false`.
- `knock_threshold: f32` — default `0.18`. Calibration overwrites.
- `knock_input_device_id: Option<String>` — optional override; default
  prefers the built-in MacBook mic, falls back to system default.
- `knock_calibration_completed: bool` — surfaced in the UI to nudge
  first-time enablers.

## Lifecycle

```
disabled         → no stream, no thread, zero CPU
enabled (idle)   → 1 cpal input stream on built-in mic, ~5 µs of work
                   per 16 ms buffer; mic indicator stays lit
recording active → tap stream keeps listening; detector still runs;
                   second double-tap fires stop_recording
calibrating      → same stream, but samples are also routed into the
                   Calibration session; UI shows 1/3 → 2/3 → 3/3
```

When the toggle flips OFF, `KnockService::stop()` drops the stream
and the worker thread exits.

## Known limitations

- **macOS mic indicator stays lit** while enabled. Documented in the
  setting's description.
- **False positives are possible** for sharp transient sounds: cup
  on desk, dropped phone, door close, hand clap. Calibration helps
  but cannot eliminate them — the conservative defaults bias toward
  missing real taps over phantom triggers.
- **Built-in mic must be available.** If the user disables the
  built-in mic at the OS level the service falls back to the default
  input device.
- **Two concurrent input streams** during recording (knock + STT) eat
  slightly more CPU than just STT alone. Acceptable; both are
  lightweight.
- **macOS only in v1.** The settings exist on all platforms (so a
  cross-device sync doesn't lose the value); the service stub returns
  an error on Windows/Linux and the UI shows the toggle disabled.

## Tests

`detector.rs` ships pure-function tests for the seven primary cases
(double-tap-confirms, gap-too-wide, typing-reject, voice-reject,
cooldown, soft-tap-ignored, post-cooldown-resume).

`calibration.rs` tests the threshold formula on a synthetic session
of three clean double-taps plus an ambient pad.

The audio engine itself is not covered by automated tests — it
requires a live mic. Manual QA checklist lives in the PR description
when this ships.
