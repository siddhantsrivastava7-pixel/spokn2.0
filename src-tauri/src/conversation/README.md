# Conversation Mode

Hands-free dictation loop inside whitelisted macOS chat apps. Listens
for the user's speech via VAD, transcribes each utterance on a brief
pause, inserts it into the focused chat input, and (optionally) auto-
sends after a countdown. Loops until the user disables it or the
focused app changes.

This module is the Rust orchestration layer. It does **not** implement
a parallel STT pipeline — the actual recording / transcription / paste
all reuse [`AudioRecordingManager`](../managers/audio.rs),
[`TranscriptionManager`](../managers/transcription.rs) and
[`clipboard::paste`](../clipboard.rs).

## Module layout

| File | Role |
| --- | --- |
| `controller.rs` | Pure state machine — `(ControllerCore, Event, Action)`. No audio, no Tauri. Fully unit-tested. |
| `app_detector.rs` | Polls macOS `NSWorkspace.frontmostApplication` and classifies bundle ids against a 6-app chat whitelist. |
| `vad_loop.rs` | Owns a low-power cpal stream + Silero VAD. Emits `on_speech_start` / `on_speech_end` edges. macOS-only audio engine. |
| `driver.rs` | Tauri-aware glue. Wires app detector + VAD loop into the controller, runs `Action`s, emits `conversation-state-changed` events. |

## State machine (overview)

```
                     ┌─────────────────────────────────┐
                     │              Off                │  ← Conversation Mode toggle OFF
                     └───────────────┬─────────────────┘
                                     │ Enable
                                     ▼
       ┌─────────────────────────────────────────────────┐
       │        PausedUnsupportedApp(focused_id?)        │  ← waiting for chat app focus
       └───────────────┬─────────────────────────────────┘
       app focused on supported chat (Messages / WhatsApp / …)
                       │
                       ▼
       ┌──────────────────────────────────┐
       │            Listening             │ ◀──── (return after each loop)
       └───────────────┬──────────────────┘
                       │ VAD: speech onset (>=200ms loud)
                       ▼
       ┌──────────────────────────────────┐
       │            Recording             │
       └───────────────┬──────────────────┘
                       │ VAD: silence ≥1100ms  OR  utterance > max
                       ▼
       ┌──────────────────────────────────┐
       │           Transcribing           │
       └────┬───────────────────────┬─────┘
            │ focus still chat       │ focus left chat app
            ▼                        ▼
   Paste → Listening         ReadyToInsert(transcript)
   (or → SendingIn(N)        ↓ user clicks Insert / Discard
    if Chat Mode on)          → Listening
```

Cancellation events that hard-reset to `Off`: user toggles off,
recording error, mic permission revoked.

Cancellation events that pause without losing the transcript: focus
leaves a supported app *during transcription* (we hold the result in
`ReadyToInsert` for the user to act on).

## Whitelist (v1)

Bundle ids matched in `app_detector::classify_bundle_id`:

| App | Bundle id |
| --- | --- |
| Messages | `com.apple.MobileSMS` |
| WhatsApp Desktop | `net.whatsapp.WhatsApp` |
| Telegram Desktop | `ru.keepcoder.Telegram`, `org.telegram.desktop` |
| Signal | `org.whispersystems.signal-desktop` |
| Slack Desktop | `com.tinyspeck.slackmacgap` |
| Discord | `com.hnc.Discord` |

Browser-based clients (WhatsApp Web, Slack web) are intentionally
**out of scope for v1** — they require browser URL probing via the
Accessibility API which is fragile and merits its own design pass.

## VAD parameters

| Constant | Value | Reason |
| --- | --- | --- |
| `SAMPLE_RATE` | 16 kHz | Silero's expected rate. Higher rates from the device are linearly decimated to this. |
| `FRAME_SAMPLES` | 480 | 30 ms — Silero's frame size. |
| `SPEECH_ONSET_MS` | 200 | Speech must persist this long before we mark the utterance "started" — guards against single-frame false positives. |
| `SILENCE_HANGOVER_MS` | 1100 | Silence must persist this long before we mark the utterance "ended". Sits in the middle of the spec's 900–1400 window. |
| `MIN_UTTERANCE_MS` | 500 | Reject utterances shorter than this — usually a chair creak or cough that briefly tripped the VAD. |
| `max_utterance_ms` | 45 000 (settings) | Hard cap. Beyond this the VAD loop force-ends the utterance even if speech continues. |

## Settings

Persisted in `AppSettings`:

- `conversation_mode_enabled: bool` — default `false`.
- `chat_mode_enabled: bool` — default `false`. Distinct from the legacy `auto_submit` toggle.
- `chat_mode_countdown_secs: u8` — default `3`. Allowed: 1, 2, 3, 5.
- `conversation_max_utterance_ms: u32` — default `45_000`.

## Lifecycle

```
disabled         → no app detector, no VAD stream, no audio engine
enabled (idle)   → AppDetector polling NSWorkspace every 500ms
                   VAD stream open on default mic
                   No recording until VAD detects speech
recording        → AudioRecordingManager opened with binding
                   "conversation_mode"; existing transcription
                   pipeline fires when VAD calls SpeechEnd
chat mode        → After paste, 1-Hz countdown timer ticks down to 0
                   then `clipboard::send_chat_mode_enter` fires
                   the user's configured Enter / Ctrl+Enter / Cmd+Enter
```

The macOS mic indicator stays lit continuously while Conversation
Mode is on (same as Knock Mode and any other always-listening
service).

## What's intentionally NOT in v1

- Browser app support (WhatsApp Web, Slack web, etc.).
- Per-field detection (we only check bundle id; clicking inside a
  non-text region of a supported chat app still leaves the mode
  active).
- Hotkey toggle for Conversation Mode (settings UI only).
- A dedicated floating overlay window for the action panel — for v1
  the panel renders inside the main app window. The familiar
  recording-pill overlay covers the passive states (Listening /
  Recording / Transcribing) so users only see *one* overlay at a
  time. Promote the action panel to a dedicated NSPanel in v2.

## Two-tier visibility

```
state               surface
─────               ───────
Off                 nothing
Listening           recording pill (existing overlay)
Recording           recording pill (existing overlay)
Transcribing        transcribing pill (existing overlay)
SendingIn(N)        action panel  (Send now / Cancel send)
ReadyToInsert       action panel  (Insert / Discard)
PausedUnsupportedApp action panel  (nudge to switch apps)
PausedByUser        action panel  (Resume button)
Error               action panel  (reason + Stop)
```

Rationale: passive states already have a perfect indicator in the
existing pill — adding a second always-on panel would clutter the
chat window. The panel only appears when the user must (or might
want to) take an action.
- Per-app "Always on top" enforcement.

## Tests

`controller.rs` ships pure-function tests for the 11 primary
behaviours (enable, supported→listening, unsupported refuses,
speech start/end transitions, chat-mode countdown, cancel-send,
focus-lost-stops-recording, empty-transcript-recovers, disable-from-
anywhere, pause/resume round-trip).

`app_detector.rs` tests the bundle-id whitelist for all six supported
apps and explicit rejection of common non-chat apps (Chrome, Safari,
Mail, Notion, VSCode, Terminal, garbage strings).

The audio engine itself is not unit-tested — it requires a live mic.
Manual QA checklist:

- WhatsApp / iMessage: speak → pause → transcript inserts; if Chat
  Mode is on, countdown starts and Enter fires at 0.
- Switch to Gmail mid-recording → mode pauses, no auto-paste.
- Open Notes (unsupported) and dictate: nothing happens.
- Toggle off → mic indicator goes dark within ~1 s.
- CPU stays under 2% on idle Listening.
