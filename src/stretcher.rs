#![doc=include_str!("../README.md")]

use bungee_sys::BungeeStretcher;

pub use crate::{InputChunk, OutputChunk, Request};

// -------------------------------------------------------------------------------------------------

/// A safe wrapper around the Bungee stretcher instance.
pub struct Stretcher {
    inner: *mut BungeeStretcher,
    sample_rate: usize,
    num_channels: usize,
}

unsafe impl Send for Stretcher {}
unsafe impl Sync for Stretcher {}

impl Stretcher {
    /// Creates and initializes a Bungee stretcher instance.
    ///
    /// # Returns
    /// A `Stretcher` instance or an error if the stretcher cannot be created.
    ///
    /// # Errors
    /// Returns an error if the sample rate or channel count is invalid, or if the
    /// Bungee function table cannot be retrieved or the stretcher cannot be created.
    pub fn new(sample_rate: usize, num_channels: usize) -> Result<Self, &'static str> {
        if sample_rate == 0 {
            return Err("Invalid sample rate");
        }
        if num_channels == 0 {
            return Err("Invalid channel count");
        }
        let sample_rates = bungee_sys::SampleRates {
            input: sample_rate as i32,
            output: sample_rate as i32,
        };

        // The C++ API defaults log2SynthesisHopAdjust to 0
        let log2_hop_adjust = 0;

        let inner =
            bungee_sys::stretcher::create(sample_rates, num_channels as i32, log2_hop_adjust);
        if inner.is_null() {
            return Err("Failed to create Bungee stretcher");
        }

        Ok(Stretcher {
            inner,
            sample_rate,
            num_channels,
        })
    }

    /// Returns the stretcher's C++ handle.  
    pub(crate) fn inner(&self) -> *mut BungeeStretcher {
        self.inner
    }

    /// Returns the stretcher's sample rate.  
    pub fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    /// Returns the stretcher's channel layout.  
    pub fn num_channels(&self) -> usize {
        self.num_channels
    }

    /// Returns the largest number of frames that might be requested by specify_grain().
    /// This helps the caller to allocate large enough buffers because it is guaranteed that
    /// `InputChunk.len()` will not exceed this number.
    pub fn max_input_frame_count(&self) -> usize {
        bungee_sys::stretcher::max_input_frame_count(self.inner) as usize
    }

    /// Adjusts `request.position` for a run-in.
    pub fn preroll(&mut self, request: &mut Request) {
        let mut ffi_request: bungee_sys::Request = (*request).into();
        bungee_sys::stretcher::preroll(self.inner, &mut ffi_request);
        *request = ffi_request.into();
    }

    /// Specifies a grain and computes the necessary input audio segment.
    pub fn specify_grain(&mut self, request: &Request) -> InputChunk {
        let ffi_request: bungee_sys::Request = (*request).into();
        // The C++ API defaults bufferStartPosition to 0.0, so we do the same.
        let buffer_start_pos = 0.0;
        bungee_sys::stretcher::specify_grain(self.inner, &ffi_request, buffer_start_pos).into()
    }

    /// Begins processing the grain with the provided audio data.
    ///
    /// # Safety
    /// `data` must be a valid pointer to audio data corresponding to the chunk specified
    /// by a prior call to `specify_grain`.
    pub fn analyse_grain(&mut self, data: &mut [f32], channel_stride: usize) {
        // The C++ API defaults mute counts to 0, so we do the same.
        let mute_head = 0;
        let mute_tail = 0;
        bungee_sys::stretcher::analyse_grain(
            self.inner,
            data.as_ptr(),
            channel_stride as isize,
            mute_head,
            mute_tail,
        );
    }

    /// Completes processing of the grain and writes the output.
    pub fn synthesise_grain(&mut self, output: &mut OutputChunk) {
        let mut ffi_output: bungee_sys::OutputChunk = output.into();
        bungee_sys::stretcher::synthesise_grain(self.inner, &mut ffi_output);
        *output = ffi_output.into();
    }

    /// Prepares `request.position` and `request.reset` for the subsequent grain.
    pub fn next(&mut self, request: &mut Request) {
        let mut ffi_request = (*request).into();
        bungee_sys::stretcher::next(self.inner, &mut ffi_request);
        *request = ffi_request.into();
    }

    /// Returns true (non-zero) if the stretcher's pipeline is flushed.
    pub fn is_flushed(&self) -> bool {
        bungee_sys::stretcher::is_flushed(self.inner) != 0
    }
}

impl Drop for Stretcher {
    /// Destroys a Bungee stretcher instance and frees its memory.
    fn drop(&mut self) {
        bungee_sys::stretcher::destroy(self.inner);
    }
}

// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stretcher_processing() {
        // Test the complete workflow with a simple sine wave
        let mut stretcher = Stretcher::new(44100, 1).unwrap();

        let mut request = Request {
            pitch: 1.0,
            speed: 1.0,
            position: 0.0,
            reset: false,
        };

        // Preroll
        stretcher.preroll(&mut request);

        // Specify grain
        let input_chunk = stretcher.specify_grain(&request);
        let sample_count = (input_chunk.end - input_chunk.begin).max(0) as usize;

        // Create a simple sine wave for testing
        let mut data = vec![0.0f32; sample_count];
        for (i, sample) in data.iter_mut().enumerate().take(sample_count) {
            *sample = (i as f32 * 0.1).sin(); // Simple sine wave
        }

        // Analyse grain with sine wave data
        stretcher.analyse_grain(&mut data, 1);

        // Synthesise grain
        let mut output_data = vec![0.0f32; 1024]; // Allocate buffer for output
        let mut output = OutputChunk::new(&mut output_data, 1);
        stretcher.synthesise_grain(&mut output);

        // Next
        stretcher.next(&mut request);

        // Verify we didn't panic and the request was updated
        assert!(request.position >= 0.0);
        assert!(output.frame_count > 0);
    }
}
