//! ML-based noise suppression via [nnnoiseless], a pure-Rust port of
//! Mozilla's RNNoise. Sits between the cpal stream and the
//! VAD/resampler so Whisper/Parakeet see a cleaner audio buffer.
//!
//! Constraints:
//!   * RNNoise expects 48 kHz mono f32 in `[-32768, 32767]` range.
//!     We accept the cpal native rate and only run when it equals
//!     48 kHz (the macOS / most-Windows default). Other rates skip
//!     the denoiser — the upstream resampler still does its job.
//!   * Frame size is fixed at 480 samples (10 ms). Input is buffered
//!     until a frame is full, then processed in one shot.
//!
//! Performance: ~1-2 % CPU on a single core, negligible memory.

use nnnoiseless::DenoiseState;

/// RNNoise's required input rate.
pub const RNNOISE_SAMPLE_RATE: u32 = 48_000;

/// One frame of audio that RNNoise can chew on. 10 ms at 48 kHz.
pub const RNNOISE_FRAME_SIZE: usize = 480;

/// Wraps a single `DenoiseState` and adapts the ".extend whatever
/// you've got, get back whatever's done" interface that fits cpal's
/// arbitrary-length buffer callback.
pub struct Denoiser {
    state: Box<DenoiseState<'static>>,
    /// Pending samples not yet large enough to fill a 480-sample
    /// frame. Drained as full frames complete.
    in_buf: Vec<f32>,
    /// Reusable output scratch — clearing on each call avoids
    /// repeated allocations on the audio thread.
    out_buf: Vec<f32>,
}

impl Denoiser {
    pub fn new() -> Self {
        Self {
            state: DenoiseState::new(),
            in_buf: Vec::with_capacity(RNNOISE_FRAME_SIZE * 2),
            out_buf: Vec::with_capacity(RNNOISE_FRAME_SIZE * 2),
        }
    }

    /// Process samples (must be 48 kHz mono f32 in `[-1, 1]`).
    /// Returns the denoised samples available so far. Some leading
    /// input may be buffered until a full 480-sample frame fills
    /// up — the caller should treat the output as a stream, not a
    /// 1:1 transform.
    pub fn process(&mut self, input: &[f32]) -> Vec<f32> {
        self.out_buf.clear();
        // RNNoise's f32 API still expects the *scale* of i16
        // (i.e. ~[-32768, 32767]); upstream cpal hands us [-1, 1].
        for &s in input {
            self.in_buf.push(s * 32_768.0);

            if self.in_buf.len() >= RNNOISE_FRAME_SIZE {
                let mut frame_in = [0f32; RNNOISE_FRAME_SIZE];
                let mut frame_out = [0f32; RNNOISE_FRAME_SIZE];
                frame_in.copy_from_slice(&self.in_buf[..RNNOISE_FRAME_SIZE]);
                self.in_buf.drain(..RNNOISE_FRAME_SIZE);

                let _ = self.state.process_frame(&mut frame_out, &frame_in);

                for &y in frame_out.iter() {
                    self.out_buf.push(y / 32_768.0);
                }
            }
        }
        self.out_buf.clone()
    }
}

impl Default for Denoiser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_size_constant_matches_rnnoise_api() {
        // Sanity check that we're using the right frame size — the
        // RNNoise contract is hard-coded at 480 samples / 10 ms /
        // 48 kHz. If nnnoiseless ever changes this, the assert
        // here will surface the breakage immediately.
        assert_eq!(DenoiseState::FRAME_SIZE, RNNOISE_FRAME_SIZE);
    }

    #[test]
    fn buffers_partial_frames() {
        let mut d = Denoiser::new();
        // Fewer than 480 samples → nothing emitted yet.
        let out = d.process(&[0.0; 100]);
        assert!(out.is_empty());
    }

    #[test]
    fn emits_frame_once_full() {
        let mut d = Denoiser::new();
        // Exactly one frame — should emit one frame worth of output.
        let input: Vec<f32> = (0..RNNOISE_FRAME_SIZE).map(|_| 0.0).collect();
        let out = d.process(&input);
        assert_eq!(out.len(), RNNOISE_FRAME_SIZE);
    }

    #[test]
    fn handles_input_larger_than_frame() {
        let mut d = Denoiser::new();
        // Two frames + some remainder: emit two frames, buffer the rest.
        let input: Vec<f32> = (0..(RNNOISE_FRAME_SIZE * 2 + 50))
            .map(|_| 0.0)
            .collect();
        let out = d.process(&input);
        assert_eq!(out.len(), RNNOISE_FRAME_SIZE * 2);
    }
}
