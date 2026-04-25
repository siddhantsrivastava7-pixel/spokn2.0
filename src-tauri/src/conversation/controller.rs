//! Conversation Mode state machine.
//!
//! Pure orchestration — no audio, no transcription. The actual recording
//! is delegated to [`crate::managers::audio::AudioRecordingManager`] and
//! the transcription/paste pipeline runs unchanged via the existing
//! `TranscribeAction` machinery. The controller's job is to:
//!
//!   * track which logical state we're in,
//!   * coalesce events from the VAD loop, the app detector, and the UI,
//!   * gate behaviour on whether the focused app is supported,
//!   * emit `conversation-state-changed` events the overlay can render,
//!   * arm / cancel the Chat Mode countdown when an utterance lands.
//!
//! Designed so the entire state machine is unit-testable without Tauri:
//! a `ControllerCore` value advances on `Event::*` calls and returns
//! the actions a real-world driver should take next.

/// What the user / overlay sees. Mirror of the spec's overlay states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationState {
    /// Conversation Mode is OFF — controller is dormant.
    Off,
    /// Mode is on but the focused app isn't on the whitelist (or no
    /// app is focused yet). We hold here until the user switches into
    /// a supported chat app.
    PausedUnsupportedApp { focused_bundle_id: Option<String> },
    /// Manually paused by the user (Pause button).
    PausedByUser,
    /// Listening for the next utterance via VAD.
    Listening,
    /// Speech detected; AudioRecordingManager is capturing.
    Recording,
    /// Recording stopped, transcription in flight.
    Transcribing,
    /// Transcription complete but app/focus is no longer valid;
    /// transcript held in the overlay for the user to act on.
    ReadyToInsert { transcript: String },
    /// Chat Mode countdown active before auto-sending.
    SendingIn { transcript: String, secs_left: u8 },
    /// A recoverable failure — transcript may be present or empty.
    Error { reason: String },
}

impl ConversationState {
    pub fn label(&self) -> &'static str {
        match self {
            ConversationState::Off => "off",
            ConversationState::PausedUnsupportedApp { .. } => "paused_unsupported_app",
            ConversationState::PausedByUser => "paused_by_user",
            ConversationState::Listening => "listening",
            ConversationState::Recording => "recording",
            ConversationState::Transcribing => "transcribing",
            ConversationState::ReadyToInsert { .. } => "ready_to_insert",
            ConversationState::SendingIn { .. } => "sending_in",
            ConversationState::Error { .. } => "error",
        }
    }
}

/// Inputs into the state machine. Each of these is something the
/// driver (the real Tauri-aware glue) feeds in as it observes the
/// world.
#[derive(Debug, Clone)]
pub enum Event {
    /// User flipped the toggle ON.
    Enable,
    /// User flipped the toggle OFF.
    Disable,
    /// User pressed Pause in the overlay.
    PauseRequested,
    /// User pressed Resume in the overlay.
    ResumeRequested,
    /// User pressed Stop in the overlay (= Disable + UI confirmation).
    StopRequested,
    /// AppDetector observed the frontmost-app change.
    AppFocusChanged {
        is_supported: bool,
        bundle_id: Option<String>,
    },
    /// VAD loop detected the start of an utterance.
    SpeechStart,
    /// VAD loop detected end-of-utterance (silence sustained).
    SpeechEnd,
    /// Recording was explicitly stopped by the controller (max-len cap).
    MaxUtteranceReached,
    /// Transcription finished AND the focused app is still a
    /// supported chat app (driver pre-checks before firing).
    /// `ok=false` or empty `transcript` means we drop and resume.
    TranscriptionDone { transcript: String, ok: bool },
    /// Transcription finished but focus left the supported chat app
    /// while we were transcribing. The controller routes to
    /// `ReadyToInsert` instead of pasting blind.
    TranscriptionDoneFocusLost { transcript: String },
    /// Tick from a 1-second timer while in `SendingIn`.
    CountdownTick,
    /// User pressed "Send now" in the overlay (skip the rest of the
    /// countdown and send immediately).
    ForceSend,
    /// User pressed "Cancel send" — keeps the transcript but skips
    /// auto-send. Returns to Listening (or ReadyToInsert if app went
    /// away during countdown).
    CancelSend,
    /// User pressed Insert in the ReadyToInsert overlay.
    InsertPending,
    /// User pressed Discard in the ReadyToInsert overlay.
    DiscardPending,
    /// Recording failed (mic permission, no input device, etc).
    RecordingError { detail: String },
}

/// Actions the controller asks the driver to perform after applying
/// an event. Pure data — no side effects until the driver runs them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Begin AudioRecordingManager capture with the given binding id.
    StartRecording,
    /// Stop AudioRecordingManager capture (samples flow into the
    /// existing transcription pipeline).
    StopRecording,
    /// Insert the transcript into the focused app via the existing
    /// paste pipeline.
    Paste { text: String },
    /// Auto-press Enter (reuses the existing `auto_submit_key` logic).
    SendReturn,
    /// Copy the transcript to the user's clipboard (Copy button in
    /// ReadyToInsert).
    CopyToClipboard { text: String },
    /// Start a 1 Hz countdown timer that emits `CountdownTick` events.
    StartCountdownTimer,
    /// Cancel the countdown timer.
    StopCountdownTimer,
    /// Start the audio engine (cpal stream + VAD loop).
    StartAudioEngine,
    /// Stop the audio engine.
    StopAudioEngine,
    /// Start polling the frontmost-app detector.
    StartAppDetector,
    /// Stop polling the frontmost-app detector.
    StopAppDetector,
    /// Push a state-changed notification (driver emits Tauri event).
    EmitStateChanged,
}

/// Pure state machine. Owns the [`ConversationState`] + Chat Mode
/// configuration; driver feeds events and runs the actions returned.
#[derive(Debug, Clone)]
pub struct ControllerCore {
    state: ConversationState,
    pub chat_mode_enabled: bool,
    pub countdown_secs: u8,
}

impl ControllerCore {
    pub fn new(chat_mode_enabled: bool, countdown_secs: u8) -> Self {
        Self {
            state: ConversationState::Off,
            chat_mode_enabled,
            countdown_secs: countdown_secs.clamp(1, 5),
        }
    }

    pub fn state(&self) -> &ConversationState {
        &self.state
    }

    pub fn is_active(&self) -> bool {
        !matches!(self.state, ConversationState::Off)
    }

    pub fn handle(&mut self, ev: Event) -> Vec<Action> {
        let mut actions = Vec::new();
        let prev = self.state.clone();

        match (&self.state.clone(), ev) {
            // Toggle on — figure out where we land based on focused app.
            (ConversationState::Off, Event::Enable) => {
                self.state = ConversationState::PausedUnsupportedApp {
                    focused_bundle_id: None,
                };
                actions.push(Action::StartAppDetector);
                actions.push(Action::StartAudioEngine);
            }

            // Hard disable / stop — wipe state back to Off no matter where.
            (_, Event::Disable) | (_, Event::StopRequested) => {
                if matches!(self.state, ConversationState::SendingIn { .. }) {
                    actions.push(Action::StopCountdownTimer);
                }
                if matches!(self.state, ConversationState::Recording) {
                    actions.push(Action::StopRecording);
                }
                actions.push(Action::StopAudioEngine);
                actions.push(Action::StopAppDetector);
                self.state = ConversationState::Off;
            }

            // App focus changed.
            (_, Event::AppFocusChanged { is_supported, bundle_id }) => {
                if matches!(self.state, ConversationState::Off) {
                    // ignore focus events while disabled
                } else if is_supported {
                    // Re-arm into Listening only from a paused-by-app
                    // state. PausedByUser stays paused. Active states
                    // (Recording/Transcribing/etc) stay where they are.
                    if matches!(self.state, ConversationState::PausedUnsupportedApp { .. }) {
                        self.state = ConversationState::Listening;
                    }
                } else {
                    // Focus left a supported app: cancel any in-flight
                    // recording / countdown and pause. Do NOT discard
                    // an in-flight transcription — let it land into
                    // ReadyToInsert (handled by Transcribing path).
                    if matches!(self.state, ConversationState::Recording) {
                        actions.push(Action::StopRecording);
                    }
                    if matches!(self.state, ConversationState::SendingIn { .. }) {
                        actions.push(Action::StopCountdownTimer);
                    }
                    self.state = match &self.state {
                        // Preserve the held transcript through the
                        // pause so the user can still act on it once
                        // a supported app comes back.
                        ConversationState::ReadyToInsert { .. }
                        | ConversationState::Transcribing
                        | ConversationState::SendingIn { .. } => {
                            self.state.clone()
                        }
                        _ => ConversationState::PausedUnsupportedApp {
                            focused_bundle_id: bundle_id,
                        },
                    };
                }
            }

            (ConversationState::Listening, Event::PauseRequested) => {
                self.state = ConversationState::PausedByUser;
            }
            (ConversationState::PausedByUser, Event::ResumeRequested) => {
                self.state = ConversationState::Listening;
            }

            (ConversationState::Listening, Event::SpeechStart) => {
                self.state = ConversationState::Recording;
                actions.push(Action::StartRecording);
            }

            (ConversationState::Recording, Event::SpeechEnd)
            | (ConversationState::Recording, Event::MaxUtteranceReached) => {
                self.state = ConversationState::Transcribing;
                actions.push(Action::StopRecording);
            }

            // Transcription done. Decision tree:
            //   - empty transcript → drop and resume listening
            //   - app no longer supported → ReadyToInsert
            //   - chat mode on AND app supported → Paste + SendingIn(N)
            //   - chat mode off → Paste + Listening
            (ConversationState::Transcribing, Event::TranscriptionDone { transcript, ok }) => {
                if !ok || transcript.trim().is_empty() {
                    self.state = ConversationState::Listening;
                } else {
                    actions.push(Action::Paste {
                        text: transcript.clone(),
                    });
                    if self.chat_mode_enabled {
                        self.state = ConversationState::SendingIn {
                            transcript: transcript.clone(),
                            secs_left: self.countdown_secs,
                        };
                        actions.push(Action::StartCountdownTimer);
                    } else {
                        self.state = ConversationState::Listening;
                    }
                }
            }

            // Focus lost during transcription: hold the transcript in
            // the overlay; do NOT paste blind into whatever app the
            // user switched to. They explicitly chose option B.
            (
                ConversationState::Transcribing,
                Event::TranscriptionDoneFocusLost { transcript },
            ) => {
                if transcript.trim().is_empty() {
                    self.state = ConversationState::PausedUnsupportedApp {
                        focused_bundle_id: None,
                    };
                } else {
                    self.state = ConversationState::ReadyToInsert { transcript };
                }
            }

            // Countdown ticks — decrement; on zero, fire send.
            (ConversationState::SendingIn { transcript, secs_left }, Event::CountdownTick) => {
                let next = secs_left.saturating_sub(1);
                if next == 0 {
                    actions.push(Action::StopCountdownTimer);
                    actions.push(Action::SendReturn);
                    self.state = ConversationState::Listening;
                } else {
                    self.state = ConversationState::SendingIn {
                        transcript: transcript.clone(),
                        secs_left: next,
                    };
                }
            }

            (ConversationState::SendingIn { .. }, Event::ForceSend) => {
                actions.push(Action::StopCountdownTimer);
                actions.push(Action::SendReturn);
                self.state = ConversationState::Listening;
            }

            (ConversationState::SendingIn { .. }, Event::CancelSend) => {
                actions.push(Action::StopCountdownTimer);
                self.state = ConversationState::Listening;
            }

            (ConversationState::ReadyToInsert { transcript }, Event::InsertPending) => {
                actions.push(Action::Paste {
                    text: transcript.clone(),
                });
                self.state = ConversationState::Listening;
            }
            (ConversationState::ReadyToInsert { .. }, Event::DiscardPending) => {
                self.state = ConversationState::Listening;
            }

            (_, Event::RecordingError { detail }) => {
                if matches!(self.state, ConversationState::Recording) {
                    actions.push(Action::StopRecording);
                }
                self.state = ConversationState::Error { reason: detail };
            }

            // Anything else is a no-op for that state — explicitly
            // ignored so we don't accidentally flap.
            _ => {}
        }

        if self.state != prev {
            actions.push(Action::EmitStateChanged);
        }
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctrl(chat: bool) -> ControllerCore {
        ControllerCore::new(chat, 3)
    }

    fn supported() -> Event {
        Event::AppFocusChanged {
            is_supported: true,
            bundle_id: Some("com.apple.MobileSMS".into()),
        }
    }
    fn unsupported() -> Event {
        Event::AppFocusChanged {
            is_supported: false,
            bundle_id: Some("com.google.Chrome".into()),
        }
    }

    #[test]
    fn enable_starts_audio_and_app_detector() {
        let mut c = ctrl(false);
        let acts = c.handle(Event::Enable);
        assert!(acts.contains(&Action::StartAppDetector));
        assert!(acts.contains(&Action::StartAudioEngine));
        assert!(matches!(
            c.state(),
            ConversationState::PausedUnsupportedApp { .. }
        ));
    }

    #[test]
    fn supported_app_transitions_to_listening() {
        let mut c = ctrl(false);
        c.handle(Event::Enable);
        c.handle(supported());
        assert_eq!(c.state(), &ConversationState::Listening);
    }

    #[test]
    fn unsupported_app_refuses_listening() {
        let mut c = ctrl(false);
        c.handle(Event::Enable);
        c.handle(unsupported());
        assert!(matches!(
            c.state(),
            ConversationState::PausedUnsupportedApp { .. }
        ));
    }

    #[test]
    fn speech_start_in_listening_records() {
        let mut c = ctrl(false);
        c.handle(Event::Enable);
        c.handle(supported());
        let acts = c.handle(Event::SpeechStart);
        assert!(acts.contains(&Action::StartRecording));
        assert_eq!(c.state(), &ConversationState::Recording);
    }

    #[test]
    fn silence_stops_recording_and_transcribes() {
        let mut c = ctrl(false);
        c.handle(Event::Enable);
        c.handle(supported());
        c.handle(Event::SpeechStart);
        let acts = c.handle(Event::SpeechEnd);
        assert!(acts.contains(&Action::StopRecording));
        assert_eq!(c.state(), &ConversationState::Transcribing);
    }

    #[test]
    fn transcription_pastes_and_returns_to_listening_no_chat_mode() {
        let mut c = ctrl(false);
        c.handle(Event::Enable);
        c.handle(supported());
        c.handle(Event::SpeechStart);
        c.handle(Event::SpeechEnd);
        let acts = c.handle(Event::TranscriptionDone {
            transcript: "hello world".into(),
            ok: true,
        });
        assert!(acts.iter().any(|a| matches!(a, Action::Paste { .. })));
        assert!(!acts.contains(&Action::StartCountdownTimer));
        assert_eq!(c.state(), &ConversationState::Listening);
    }

    #[test]
    fn chat_mode_arms_countdown_and_sends_at_zero() {
        let mut c = ctrl(true);
        c.handle(Event::Enable);
        c.handle(supported());
        c.handle(Event::SpeechStart);
        c.handle(Event::SpeechEnd);
        let acts = c.handle(Event::TranscriptionDone {
            transcript: "hi".into(),
            ok: true,
        });
        assert!(acts.contains(&Action::StartCountdownTimer));
        assert!(matches!(
            c.state(),
            ConversationState::SendingIn { secs_left: 3, .. }
        ));
        c.handle(Event::CountdownTick);
        c.handle(Event::CountdownTick);
        let final_acts = c.handle(Event::CountdownTick);
        assert!(final_acts.contains(&Action::SendReturn));
        assert_eq!(c.state(), &ConversationState::Listening);
    }

    #[test]
    fn cancel_send_keeps_transcript_skips_send() {
        let mut c = ctrl(true);
        c.handle(Event::Enable);
        c.handle(supported());
        c.handle(Event::SpeechStart);
        c.handle(Event::SpeechEnd);
        c.handle(Event::TranscriptionDone {
            transcript: "hi".into(),
            ok: true,
        });
        let acts = c.handle(Event::CancelSend);
        assert!(!acts.contains(&Action::SendReturn));
        assert!(acts.contains(&Action::StopCountdownTimer));
        assert_eq!(c.state(), &ConversationState::Listening);
    }

    #[test]
    fn focus_lost_during_recording_stops_recording() {
        let mut c = ctrl(false);
        c.handle(Event::Enable);
        c.handle(supported());
        c.handle(Event::SpeechStart);
        let acts = c.handle(unsupported());
        assert!(acts.contains(&Action::StopRecording));
        // We pause the state since the recording was discarded.
        assert!(matches!(
            c.state(),
            ConversationState::PausedUnsupportedApp { .. }
        ));
    }

    #[test]
    fn empty_transcript_returns_to_listening() {
        let mut c = ctrl(true);
        c.handle(Event::Enable);
        c.handle(supported());
        c.handle(Event::SpeechStart);
        c.handle(Event::SpeechEnd);
        let acts = c.handle(Event::TranscriptionDone {
            transcript: "  ".into(),
            ok: true,
        });
        assert!(!acts.iter().any(|a| matches!(a, Action::Paste { .. })));
        assert!(!acts.contains(&Action::StartCountdownTimer));
        assert_eq!(c.state(), &ConversationState::Listening);
    }

    #[test]
    fn disable_kills_state_to_off() {
        let mut c = ctrl(true);
        c.handle(Event::Enable);
        c.handle(supported());
        c.handle(Event::SpeechStart);
        let acts = c.handle(Event::Disable);
        assert!(acts.contains(&Action::StopRecording));
        assert!(acts.contains(&Action::StopAudioEngine));
        assert!(acts.contains(&Action::StopAppDetector));
        assert_eq!(c.state(), &ConversationState::Off);
    }

    #[test]
    fn pause_then_resume_round_trips() {
        let mut c = ctrl(false);
        c.handle(Event::Enable);
        c.handle(supported());
        c.handle(Event::PauseRequested);
        assert_eq!(c.state(), &ConversationState::PausedByUser);
        c.handle(Event::ResumeRequested);
        assert_eq!(c.state(), &ConversationState::Listening);
    }
}
