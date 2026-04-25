//! Tauri-aware glue between the pure [`ControllerCore`] state machine
//! and the rest of the app. Owns the [`AppDetector`] + [`VadLoop`]
//! subscriptions, runs the [`Action`]s the controller emits, and
//! pushes `conversation-state-changed` events to the frontend.
//!
//! Recording / transcription / paste reuse the existing
//! [`crate::managers::audio::AudioRecordingManager`] +
//! [`crate::managers::transcription::TranscriptionManager`] +
//! [`crate::clipboard`] pipeline — no parallel pipeline, no
//! duplicated logic.

use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tauri::{AppHandle, Emitter, Manager};

use super::app_detector::AppDetector;
use super::controller::{Action, ControllerCore, ConversationState, Event};
use super::vad_loop::VadLoop;
use crate::managers::audio::AudioRecordingManager;
use crate::overlay::{
    hide_recording_overlay, show_recording_overlay, show_transcribing_overlay,
};
use crate::settings;

/// Identifier we use when calling `try_start_recording`. Distinct
/// from "transcribe" / "knock_mode" so debug logs and the
/// [`AudioRecordingManager`]'s state machine can distinguish them.
pub const CONVERSATION_BINDING_ID: &str = "conversation_mode";

/// One-stop façade. Lives in Tauri-managed state.
pub struct ConversationDriver {
    inner: Arc<Inner>,
}

struct Inner {
    app: AppHandle,
    core: Mutex<ControllerCore>,
    app_detector: AppDetector,
    vad_loop: VadLoop,
    countdown_thread: Mutex<Option<JoinHandle<()>>>,
    countdown_stop: Arc<std::sync::atomic::AtomicBool>,
    /// Last transcript captured from the recording pipeline. Used
    /// only when the controller asks to paste/copy/insert later.
    last_transcript: Mutex<Option<String>>,
}

impl ConversationDriver {
    pub fn new(app: AppHandle) -> Arc<Self> {
        let s = settings::get_settings(&app);
        let core = ControllerCore::new(
            s.chat_mode_enabled,
            s.chat_mode_countdown_secs,
        );

        // Build the VAD loop with closures that route into the
        // controller via `feed_event`.
        let inner_for_start = Arc::new(Mutex::new(None::<Arc<Inner>>));
        let start_inner = inner_for_start.clone();
        let on_speech_start = Arc::new(move || {
            if let Some(i) = start_inner.lock().unwrap().as_ref() {
                ConversationDriver::feed_event(i, Event::SpeechStart);
            }
        });
        let end_inner = inner_for_start.clone();
        let on_speech_end = Arc::new(move || {
            if let Some(i) = end_inner.lock().unwrap().as_ref() {
                ConversationDriver::feed_event(i, Event::SpeechEnd);
            }
        });

        let vad = VadLoop::new(
            on_speech_start,
            on_speech_end,
            s.conversation_max_utterance_ms as u64,
        );

        let inner = Arc::new(Inner {
            app: app.clone(),
            core: Mutex::new(core),
            app_detector: AppDetector::new(),
            vad_loop: vad,
            countdown_thread: Mutex::new(None),
            countdown_stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            last_transcript: Mutex::new(None),
        });
        *inner_for_start.lock().unwrap() = Some(inner.clone());

        Arc::new(Self { inner })
    }

    pub fn current_state(&self) -> ConversationState {
        self.inner.core.lock().unwrap().state().clone()
    }

    /// Push the latest Chat Mode policy into the running controller.
    /// Without this, toggling Chat Mode in settings would silently
    /// have no effect until Conversation Mode was re-enabled.
    pub fn update_chat_mode(&self, enabled: bool, countdown_secs: u8) {
        let mut core = self.inner.core.lock().unwrap();
        core.set_chat_mode_enabled(enabled);
        core.set_countdown_secs(countdown_secs);
    }

    /// Push an event in from outside. The driver applies actions and
    /// returns the (now-current) state for inspection / Tauri reply.
    pub fn dispatch(&self, ev: Event) -> ConversationState {
        Self::feed_event(&self.inner, ev);
        self.current_state()
    }

    /// Internal: apply one event + run the resulting action list.
    fn feed_event(inner: &Arc<Inner>, ev: Event) {
        let actions = {
            let mut core = inner.core.lock().unwrap();
            core.handle(ev)
        };
        for action in actions {
            Self::run_action(inner, action);
        }
    }

    fn run_action(inner: &Arc<Inner>, action: Action) {
        match action {
            Action::StartAppDetector => {
                let inner_for_cb = inner.clone();
                let _ = inner.app_detector.start(move |_prev, current| {
                    let is_supported = current.is_supported();
                    let bid = current.bundle_id().map(|s| s.to_string());
                    ConversationDriver::feed_event(
                        &inner_for_cb,
                        Event::AppFocusChanged {
                            is_supported,
                            bundle_id: bid,
                        },
                    );
                });
            }
            Action::StopAppDetector => inner.app_detector.stop(),

            Action::StartAudioEngine => {
                // Pre-load the transcription model NOW so the first
                // utterance doesn't fail with "Model is not loaded".
                // The hotkey path does this via TranscribeAction; the
                // VAD-driven path needs to do it explicitly.
                let tm = inner
                    .app
                    .state::<Arc<crate::managers::transcription::TranscriptionManager>>(
                    )
                    .inner()
                    .clone();
                tm.initiate_model_load();
                // Also pre-warm the recorder's VAD context so the
                // first try_start_recording isn't held up opening the
                // mic stream.
                let rm = inner
                    .app
                    .state::<Arc<AudioRecordingManager>>()
                    .inner()
                    .clone();
                std::thread::spawn(move || {
                    if let Err(e) = rm.preload_vad() {
                        log::debug!("Conversation Mode: preload_vad: {e}");
                    }
                });

                // Resolve the VAD model path the same way the existing
                // recorder does so we don't ship a second copy.
                let vad_path = match inner.app.path().resolve(
                    "resources/models/silero_vad_v4.onnx",
                    tauri::path::BaseDirectory::Resource,
                ) {
                    Ok(p) => p.to_string_lossy().to_string(),
                    Err(e) => {
                        log::error!("Conversation Mode: VAD path: {e}");
                        return;
                    }
                };
                if let Err(e) = inner.vad_loop.start(&vad_path) {
                    log::error!("Conversation Mode: VAD start: {e}");
                    Self::feed_event(
                        inner,
                        Event::RecordingError { detail: e },
                    );
                }
            }
            Action::StopAudioEngine => inner.vad_loop.stop(),

            Action::StartRecording => {
                let rm = inner.app.state::<Arc<AudioRecordingManager>>();
                if let Err(e) = rm.try_start_recording(CONVERSATION_BINDING_ID) {
                    log::error!("Conversation Mode: start_recording: {e}");
                    Self::feed_event(
                        inner,
                        Event::RecordingError { detail: e },
                    );
                }
            }
            Action::StopRecording => {
                // Stop the recording on a worker thread so the audio
                // callback (which may have invoked us) doesn't block.
                let rm = inner
                    .app
                    .state::<Arc<AudioRecordingManager>>()
                    .inner()
                    .clone();
                let inner_for_done = inner.clone();
                thread::spawn(move || {
                    let samples = rm.stop_recording(CONVERSATION_BINDING_ID);
                    let app = inner_for_done.app.clone();
                    let tm = app
                        .state::<Arc<crate::managers::transcription::TranscriptionManager>>(
                        )
                        .inner()
                        .clone();
                    let result = match samples {
                        Some(buf) if !buf.is_empty() => match tm.transcribe(buf) {
                            Ok(text) => (text, true),
                            Err(e) => {
                                log::warn!(
                                    "Conversation Mode: transcribe failed: {e}"
                                );
                                (String::new(), false)
                            }
                        },
                        _ => (String::new(), false),
                    };
                    *inner_for_done.last_transcript.lock().unwrap() =
                        Some(result.0.clone());

                    // Pre-check focus *before* dispatching — the
                    // controller branches on whether we should paste
                    // blind or hold the transcript in the overlay.
                    let focus = inner_for_done.app_detector.current();
                    let event = if !focus.is_supported() && result.1
                        && !result.0.trim().is_empty()
                    {
                        Event::TranscriptionDoneFocusLost {
                            transcript: result.0,
                        }
                    } else {
                        Event::TranscriptionDone {
                            transcript: result.0,
                            ok: result.1,
                        }
                    };
                    Self::feed_event(&inner_for_done, event);
                });
            }
            Action::Paste { text } => {
                let app_handle = inner.app.clone();
                thread::spawn(move || {
                    if let Err(e) = crate::clipboard::paste(text, app_handle) {
                        log::error!("Conversation Mode: paste failed: {e}");
                    }
                });
            }
            Action::SendReturn => {
                // Reuse the same low-level Enter press the existing
                // auto_submit pipeline uses. Simplest: simulate a
                // press on the configured `auto_submit_key` even if
                // `auto_submit` itself is OFF (chat_mode is the
                // independent toggle here).
                let app_handle = inner.app.clone();
                thread::spawn(move || {
                    crate::clipboard::send_chat_mode_enter(&app_handle);
                });
            }
            Action::CopyToClipboard { text } => {
                use tauri_plugin_clipboard_manager::ClipboardExt;
                if let Err(e) = inner.app.clipboard().write_text(&text) {
                    log::error!("Conversation Mode: clipboard write: {e}");
                }
            }
            Action::StartCountdownTimer => {
                Self::cancel_countdown(inner);
                inner
                    .countdown_stop
                    .store(false, std::sync::atomic::Ordering::Relaxed);
                let stop = inner.countdown_stop.clone();
                let inner_clone = inner.clone();
                let h = thread::spawn(move || loop {
                    thread::sleep(Duration::from_secs(1));
                    if stop.load(std::sync::atomic::Ordering::Relaxed) {
                        return;
                    }
                    ConversationDriver::feed_event(&inner_clone, Event::CountdownTick);
                });
                *inner.countdown_thread.lock().unwrap() = Some(h);
            }
            Action::StopCountdownTimer => Self::cancel_countdown(inner),

            Action::EmitStateChanged => {
                let core = inner.core.lock().unwrap();
                let state = core.state().clone();
                drop(core);
                // Two-tier overlay: passive states reuse the existing
                // recording pill (familiar bottom-center indicator);
                // action-required states surface via the conversation
                // panel emitted below. The pill and the panel never
                // overlap by design.
                match &state {
                    ConversationState::Listening
                    | ConversationState::Recording => {
                        show_recording_overlay(&inner.app);
                    }
                    ConversationState::Transcribing => {
                        show_transcribing_overlay(&inner.app);
                    }
                    _ => {
                        // SendingIn / ReadyToInsert / Error / Paused* /
                        // Off — pill is hidden; either nothing visible
                        // (Off / PausedByUser) or the conversation
                        // panel handles the state.
                        hide_recording_overlay(&inner.app);
                    }
                }
                emit_state(&inner.app, &state);
            }
        }
    }

    fn cancel_countdown(inner: &Inner) {
        inner
            .countdown_stop
            .store(true, std::sync::atomic::Ordering::Relaxed);
        if let Some(h) = inner.countdown_thread.lock().unwrap().take() {
            // Don't block shutdown — the thread will see the flag and exit.
            let _ = h.join();
        }
    }
}

fn emit_state(app: &AppHandle, state: &ConversationState) {
    #[derive(serde::Serialize, Clone)]
    struct Payload {
        label: String,
        transcript: Option<String>,
        secs_left: Option<u8>,
        focused_bundle_id: Option<String>,
        reason: Option<String>,
    }
    let payload = match state {
        ConversationState::ReadyToInsert { transcript } => Payload {
            label: state.label().into(),
            transcript: Some(transcript.clone()),
            secs_left: None,
            focused_bundle_id: None,
            reason: None,
        },
        ConversationState::SendingIn {
            transcript,
            secs_left,
        } => Payload {
            label: state.label().into(),
            transcript: Some(transcript.clone()),
            secs_left: Some(*secs_left),
            focused_bundle_id: None,
            reason: None,
        },
        ConversationState::PausedUnsupportedApp { focused_bundle_id } => Payload {
            label: state.label().into(),
            transcript: None,
            secs_left: None,
            focused_bundle_id: focused_bundle_id.clone(),
            reason: None,
        },
        ConversationState::Error { reason } => Payload {
            label: state.label().into(),
            transcript: None,
            secs_left: None,
            focused_bundle_id: None,
            reason: Some(reason.clone()),
        },
        _ => Payload {
            label: state.label().into(),
            transcript: None,
            secs_left: None,
            focused_bundle_id: None,
            reason: None,
        },
    };
    let _ = app.emit("conversation-state-changed", payload);
}
