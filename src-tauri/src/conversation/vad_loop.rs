//! Continuous low-power VAD loop for Conversation Mode.
//!
//! Owns its own cpal input stream + a [`SileroVad`]; emits two
//! callbacks the controller cares about: `on_speech_start` and
//! `on_speech_end`. The actual audio capture for transcription is
//! delegated to [`AudioRecordingManager`] — this loop is purely a
//! voice-activity edge detector.
//!
//! Why a separate stream?
//! Conversation Mode needs continuous mic energy analysis even when
//! the user isn't actively recording (so we know *when* to start a
//! recording). Reusing AudioRecordingManager's stream would require
//! switching it to AlwaysOn mode and intertwining VAD subscribers
//! with its existing level-callback machinery — more refactor risk.
//! The cost is one extra cpal stream while Conversation Mode is on,
//! which on macOS shares the single mic indicator anyway.
//!
//! Same architecture as `tap_detection::service` — macOS-only audio
//! engine; non-macOS builds compile to a stub so the rest of the
//! controller doesn't need cfg-guards.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

use crate::audio_toolkit::vad::{SileroVad, VoiceActivityDetector};

/// 30 ms at 16 kHz — matches Silero's expected frame.
const SAMPLE_RATE: u32 = 16_000;
const FRAME_SAMPLES: usize = 480;

/// Speech must persist this long before we consider the utterance
/// "started" — guards against single-frame false positives.
const SPEECH_ONSET_MS: u64 = 200;

/// Silence must persist this long before we consider the utterance
/// "ended". 1100 ms sits in the middle of the 900–1400 spec window.
const SILENCE_HANGOVER_MS: u64 = 1100;

/// Reject utterances shorter than this — usually a chair creak or
/// throat-clear that briefly tripped the VAD.
const MIN_UTTERANCE_MS: u64 = 500;

pub type EdgeCallback = Arc<dyn Fn() + Send + Sync + 'static>;

/// Public façade. Always constructible; only opens audio on `start()`,
/// and only on macOS. Non-macOS callers receive an error from `start`.
pub struct VadLoop {
    inner: Arc<Inner>,
    #[cfg(target_os = "macos")]
    stream: Mutex<Option<cpal::Stream>>,
}

struct Inner {
    on_speech_start: EdgeCallback,
    on_speech_end: EdgeCallback,
    /// Hard cap on a single utterance — the audio worker emits
    /// `on_speech_end` early when this is reached even if speech
    /// hasn't fallen silent.
    max_utterance_ms: AtomicU64,
    stop_flag: Arc<AtomicBool>,
}

// Safety: the cpal::Stream is !Send because the underlying CoreAudio
// resource is tied to a thread; we never share it across threads
// (constructed and dropped from the Tauri-managed thread). Same
// rationale as KnockService.
#[cfg(target_os = "macos")]
unsafe impl Send for VadLoop {}
#[cfg(target_os = "macos")]
unsafe impl Sync for VadLoop {}

impl VadLoop {
    pub fn new(
        on_speech_start: EdgeCallback,
        on_speech_end: EdgeCallback,
        max_utterance_ms: u64,
    ) -> Self {
        Self {
            inner: Arc::new(Inner {
                on_speech_start,
                on_speech_end,
                max_utterance_ms: AtomicU64::new(max_utterance_ms),
                stop_flag: Arc::new(AtomicBool::new(false)),
            }),
            #[cfg(target_os = "macos")]
            stream: Mutex::new(None),
        }
    }

    pub fn set_max_utterance_ms(&self, ms: u64) {
        self.inner.max_utterance_ms.store(ms, Ordering::Relaxed);
    }

    #[cfg(target_os = "macos")]
    pub fn start(&self, vad_model_path: &str) -> Result<(), String> {
        let mut guard = self.stream.lock().unwrap();
        if guard.is_some() {
            return Ok(());
        }
        self.inner.stop_flag.store(false, Ordering::Relaxed);
        let stream = open_stream(self.inner.clone(), vad_model_path)?;
        *guard = Some(stream);
        log::info!("Conversation Mode: VAD loop started");
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    pub fn start(&self, _vad_model_path: &str) -> Result<(), String> {
        Err("Conversation Mode is only available on macOS".to_string())
    }

    pub fn stop(&self) {
        self.inner.stop_flag.store(true, Ordering::Relaxed);
        #[cfg(target_os = "macos")]
        {
            // Dropping ends the audio thread.
            *self.stream.lock().unwrap() = None;
        }
        log::info!("Conversation Mode: VAD loop stopped");
    }
}

#[cfg(target_os = "macos")]
fn open_stream(
    inner: Arc<Inner>,
    vad_model_path: &str,
) -> Result<cpal::Stream, String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| "No default input device".to_string())?;
    let device_name = device.name().unwrap_or_else(|_| "<unknown>".into());
    let cfg = device
        .default_input_config()
        .map_err(|e| format!("default_input_config: {e}"))?;
    let channels = cfg.channels() as usize;
    let device_rate = cfg.sample_rate().0;
    log::info!(
        "Conversation Mode VAD on '{}' @ {} Hz, {} ch (resampling to 16k mono)",
        device_name,
        device_rate,
        channels
    );

    let mut vad = SileroVad::new(vad_model_path, 0.5)
        .map_err(|e| format!("Failed to load Silero VAD model: {e}"))?;

    // Per-stream mutable state, captured by the audio callback.
    let mut frame_buf: Vec<f32> = Vec::with_capacity(FRAME_SAMPLES);
    let mut speech_run_ms: u64 = 0;
    let mut silence_run_ms: u64 = 0;
    let mut in_speech: bool = false;
    let mut utterance_ms: u64 = 0;
    let mut samples_seen: u64 = 0;

    // Resample-on-the-fly: skip-or-duplicate is too noisy for VAD; we
    // do a simple linear decimation since we only need amplitude
    // shape, not phase fidelity. Acceptable for VAD.
    let downsample_ratio: f32 = device_rate as f32 / SAMPLE_RATE as f32;
    let mut resample_acc: f32 = 0.0;

    let err_fn = |e| log::error!("Conversation VAD stream error: {e}");

    macro_rules! build {
        ($t:ty) => {{
            let inner_cb = inner.clone();
            device
                .build_input_stream(
                    &cfg.clone().into(),
                    move |data: &[$t], _: &cpal::InputCallbackInfo| {
                        if inner_cb.stop_flag.load(Ordering::Relaxed) {
                            return;
                        }

                        // Mono mix-down + optional decimation into 16 kHz.
                        let frame_count = if channels == 0 {
                            0
                        } else {
                            data.len() / channels
                        };
                        for i in 0..frame_count {
                            let mut acc: f32 = 0.0;
                            for c in 0..channels {
                                let s: f32 = cpal::Sample::to_sample::<f32>(
                                    data[i * channels + c],
                                );
                                acc += s;
                            }
                            let mono = acc / channels.max(1) as f32;
                            // Decimate by accumulating fractional indices.
                            resample_acc += 1.0;
                            if resample_acc >= downsample_ratio {
                                resample_acc -= downsample_ratio;
                                frame_buf.push(mono);
                                samples_seen += 1;

                                if frame_buf.len() == FRAME_SAMPLES {
                                    let frame: Vec<f32> = std::mem::take(&mut frame_buf);
                                    frame_buf.reserve(FRAME_SAMPLES);
                                    let frame_ms = 30u64; // by construction
                                    let is_speech = match vad.is_voice(&frame) {
                                        Ok(v) => v,
                                        Err(e) => {
                                            log::trace!("VAD frame error: {e}");
                                            false
                                        }
                                    };

                                    if is_speech {
                                        speech_run_ms += frame_ms;
                                        silence_run_ms = 0;
                                        if in_speech {
                                            utterance_ms += frame_ms;
                                            // Hard cap on utterance length.
                                            let cap = inner_cb
                                                .max_utterance_ms
                                                .load(Ordering::Relaxed);
                                            if utterance_ms >= cap {
                                                in_speech = false;
                                                utterance_ms = 0;
                                                speech_run_ms = 0;
                                                silence_run_ms = 0;
                                                let cb = inner_cb.on_speech_end.clone();
                                                std::thread::spawn(move || (cb)());
                                            }
                                        } else if speech_run_ms >= SPEECH_ONSET_MS {
                                            in_speech = true;
                                            utterance_ms = speech_run_ms;
                                            let cb = inner_cb.on_speech_start.clone();
                                            std::thread::spawn(move || (cb)());
                                        }
                                    } else {
                                        speech_run_ms = 0;
                                        if in_speech {
                                            silence_run_ms += frame_ms;
                                            utterance_ms += frame_ms;
                                            if silence_run_ms >= SILENCE_HANGOVER_MS {
                                                if utterance_ms >= MIN_UTTERANCE_MS {
                                                    let cb = inner_cb.on_speech_end.clone();
                                                    std::thread::spawn(move || (cb)());
                                                }
                                                in_speech = false;
                                                utterance_ms = 0;
                                                silence_run_ms = 0;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        let _ = samples_seen; // silence unused on slow paths
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| format!("build_input_stream: {e}"))
        }};
    }

    let stream = match cfg.sample_format() {
        cpal::SampleFormat::F32 => build!(f32)?,
        cpal::SampleFormat::I16 => build!(i16)?,
        cpal::SampleFormat::I32 => build!(i32)?,
        cpal::SampleFormat::U16 => build!(u16)?,
        other => return Err(format!("Unsupported sample format {:?}", other)),
    };

    stream.play().map_err(|e| format!("stream.play: {e}"))?;

    // Suppress warnings about unused locals on cfgs that skip the
    // path above (none currently, but keeps future-proofing tidy).
    let _ = (Duration::from_millis(0),);

    Ok(stream)
}
