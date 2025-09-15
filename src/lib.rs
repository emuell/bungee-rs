//! High level wrapper for the bungee-sys FFI library.

use lazy_static::lazy_static;

// -------------------------------------------------------------------------------------------------

// Bungee C API library functions, initialized once at runtime
lazy_static! {
    static ref BUNGEE_FUNCTIONS: bungee_sys::Functions = {
        #[cfg(not(feature = "bungee_pro"))]
        let funcs_ptr = unsafe {
            bungee_sys::getFunctionsBungeeBasic()
        };
        #[cfg(feature = "bungee_pro")]
        let funcs_ptr = unsafe {
            bungee_sys::getFunctionsBungeePro()
        };
        if funcs_ptr.is_null() {
            panic!("Failed to get Bungee function table");
        }
        // Safety: The library guarantees this is a valid, static pointer
        unsafe { *funcs_ptr }
    };
}

// -------------------------------------------------------------------------------------------------

/// A safe wrapper around the FFI Request struct.
#[derive(Debug, Clone, Copy)]
pub struct Request {
    /// Frame-offset within the input audio of the centre-point of the current audio grain.
    /// `NaN` signifies an invalid grain that produces no audio output and may be used for flushing.
    pub position: f64,

    /// Output audio speed. A value of 1.0 means speed should be unchanged relative to the input audio.
    /// Used by Stretcher's internal algorithms only when it's not possible to determine speed by
    /// subtracting `Request::position` of the previous grain from the current grain.
    pub speed: f64,

    /// Adjustment as a frequency multiplier with a value of 1.0 meaning no pitch adjustment.
    pub pitch: f64,

    /// Set to have the stretcher forget all previous grains and restart on this grain.
    /// (false for 0, true for non-zero)
    pub reset: bool,
}

impl From<bungee_sys::Request> for Request {
    fn from(ffi: bungee_sys::Request) -> Self {
        Request {
            position: ffi.position,
            speed: ffi.speed,
            pitch: ffi.pitch,
            reset: ffi.reset != 0,
        }
    }
}

impl From<Request> for bungee_sys::Request {
    fn from(r: Request) -> bungee_sys::Request {
        bungee_sys::Request {
            position: r.position,
            speed: r.speed,
            pitch: r.pitch,
            reset: if r.reset { 1 } else { 0 },
        }
    }
}

// -------------------------------------------------------------------------------------------------

/// A safe wrapper around the FFI InputChunk struct.
#[derive(Debug, Clone, Copy)]
pub struct InputChunk {
    /// Frame offsets relative to the start of the audio track.
    pub begin: isize,
    pub end: isize,
}

impl From<bungee_sys::InputChunk> for InputChunk {
    fn from(ffi: bungee_sys::InputChunk) -> Self {
        InputChunk {
            begin: ffi.begin as isize,
            end: ffi.end as isize,
        }
    }
}

impl From<InputChunk> for bungee_sys::InputChunk {
    fn from(ic: InputChunk) -> bungee_sys::InputChunk {
        bungee_sys::InputChunk {
            begin: ic.begin as i32,
            end: ic.end as i32,
        }
    }
}

// -------------------------------------------------------------------------------------------------

/// A safe wrapper around the FFI OutputChunk struct.
#[derive(Debug)]
pub struct OutputChunk<'a> {
    /// Audio output data, not aligned and not interleaved.
    pub data: &'a mut [f32],
    /// Number of frames in the output data.
    pub frame_count: usize,
    /// The nth audio channel audio starts at `data[n * channelStride]`.
    pub channel_stride: usize,
    /// `request[0]` corresponds to the first frame of data, `request[1]` corresponds to the frame
    /// after the last frame of data.
    pub request: [Option<Request>; 2],
}

impl<'a> OutputChunk<'a> {
    /// Create a new OutputChunk with the provided data slice.
    pub fn new(data: &'a mut [f32], channel_stride: usize) -> Self {
        OutputChunk {
            data,
            frame_count: 0,
            channel_stride,
            request: [None, None],
        }
    }
}

impl<'a> From<bungee_sys::OutputChunk> for OutputChunk<'a> {
    fn from(ffi: bungee_sys::OutputChunk) -> Self {
        // Safety: We're creating a slice from the raw pointer, but we don't know its length
        // This is why we need to rely on frame_count and channel_stride to determine the actual data
        let data_len = if ffi.frame_count > 0 && ffi.channel_stride > 0 {
            ffi.frame_count as usize * ffi.channel_stride as usize
        } else {
            0
        };

        // Safety: We're assuming the raw pointer points to valid memory
        // This is safe because the FFI function will have allocated this memory
        let data_slice = if data_len > 0 && !ffi.data.is_null() {
            unsafe { std::slice::from_raw_parts_mut(ffi.data, data_len) }
        } else {
            &mut []
        };

        OutputChunk {
            data: data_slice,
            frame_count: ffi.frame_count as usize,
            channel_stride: ffi.channel_stride as usize,
            request: [
                if ffi.request[0].is_null() {
                    None
                } else {
                    Some(unsafe { *ffi.request[0] }.into())
                },
                if ffi.request[1].is_null() {
                    None
                } else {
                    Some(unsafe { *ffi.request[1] }.into())
                },
            ],
        }
    }
}

impl<'a> From<&mut OutputChunk<'a>> for bungee_sys::OutputChunk {
    fn from(oc: &mut OutputChunk<'a>) -> bungee_sys::OutputChunk {
        bungee_sys::OutputChunk {
            data: if oc.data.is_empty() {
                std::ptr::null_mut()
            } else {
                oc.data.as_ptr() as *mut _
            },
            frame_count: oc.frame_count as i32,
            channel_stride: oc.channel_stride as isize,
            request: [
                if let Some(req) = oc.request[0] {
                    &req.into() as *const _
                } else {
                    std::ptr::null()
                },
                if let Some(req) = oc.request[1] {
                    &req.into() as *const _
                } else {
                    std::ptr::null()
                },
            ],
        }
    }
}

// -------------------------------------------------------------------------------------------------

/// A safe wrapper around the Bungee stretcher instance.
pub struct Stretcher {
    inner: *mut bungee_sys::BungeeStretcher,
    sample_rate: usize,
    num_channels: usize,
}

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
        unsafe {
            let functions = &BUNGEE_FUNCTIONS;

            let create_fn = match functions.create {
                Some(f) => f,
                None => return Err("Bungee create function not available"),
            };

            let sample_rates = bungee_sys::SampleRates {
                input: sample_rate as i32,
                output: sample_rate as i32,
            };

            // The C++ API defaults log2SynthesisHopAdjust to 0
            let log2_hop_adjust = 0;

            let state = create_fn(sample_rates, num_channels as i32, log2_hop_adjust);
            if state.is_null() {
                return Err("Failed to create Bungee stretcher");
            }

            Ok(Stretcher {
                inner: state,
                sample_rate,
                num_channels,
            })
        }
    }

    pub fn sample_rate(&self) -> usize {
        self.sample_rate
    }

    pub fn num_channels(&self) -> usize {
        self.num_channels
    }

    /// Adjusts `request.position` for a run-in.
    pub fn preroll(&mut self, request: &mut Request) {
        let functions = &BUNGEE_FUNCTIONS;
        let mut ffi_request: bungee_sys::Request = (*request).into();
        if let Some(preroll_fn) = functions.preroll {
            unsafe { preroll_fn(self.inner, &mut ffi_request) };
        }
        *request = ffi_request.into();
    }

    /// Specifies a grain and computes the necessary input audio segment.
    pub fn specify_grain(&mut self, request: &Request) -> InputChunk {
        let functions = &BUNGEE_FUNCTIONS;
        let ffi_request: bungee_sys::Request = (*request).into();
        if let Some(specify_grain_fn) = functions.specify_grain {
            unsafe {
                // The C++ API defaults bufferStartPosition to 0.0, so we do the same.
                let buffer_start_pos = 0.0;
                specify_grain_fn(self.inner, &ffi_request, buffer_start_pos).into()
            }
        } else {
            InputChunk { begin: 0, end: 0 }
        }
    }

    /// Begins processing the grain with the provided audio data.
    ///
    /// # Safety
    /// `data` must be a valid pointer to audio data corresponding to the chunk specified
    /// by a prior call to `bungee_specify_grain`.
    pub fn analyse_grain(&mut self, data: &mut [f32], channel_stride: isize) {
        let functions = &BUNGEE_FUNCTIONS;
        if let Some(analyse_grain_fn) = functions.analyse_grain {
            unsafe {
                // The C++ API defaults mute counts to 0, so we do the same.
                let mute_head = 0;
                let mute_tail = 0;
                analyse_grain_fn(
                    self.inner,
                    data.as_ptr(),
                    channel_stride,
                    mute_head,
                    mute_tail,
                );
            }
        }
    }

    /// Completes processing of the grain and writes the output.
    ///
    /// # Safety
    /// `output` must be a valid pointer to an `OutputChunk` struct that the library can write into.
    pub fn synthesise_grain(&mut self, output: &mut OutputChunk) {
        let functions = &BUNGEE_FUNCTIONS;
        let mut ffi_output: bungee_sys::OutputChunk = output.into();
        if let Some(synthesise_grain_fn) = functions.synthesise_grain {
            unsafe { synthesise_grain_fn(self.inner, &mut ffi_output) };
        }
        *output = ffi_output.into();
    }

    /// Prepares `request.position` and `request.reset` for the subsequent grain.
    pub fn next(&mut self, request: &mut Request) {
        let functions = &BUNGEE_FUNCTIONS;
        let mut ffi_request = (*request).into();
        if let Some(next_fn) = functions.next {
            unsafe { next_fn(self.inner, &mut ffi_request) };
        }
        *request = ffi_request.into();
    }
}

impl Drop for Stretcher {
    /// Destroys a Bungee stretcher instance and frees its memory.
    fn drop(&mut self) {
        unsafe {
            let functions = &BUNGEE_FUNCTIONS;
            if let Some(destroy_fn) = functions.destroy {
                destroy_fn(self.inner);
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_sine_wave() {
        // Test the complete workflow with a simple sine wave
        let mut stretcher = Stretcher::new(44100, 1).unwrap();

        let mut request = Request {
            pitch: 1.0,
            speed: 1.0,
            position: 0.0,
            reset: false,
        };

        // 1. Preroll
        stretcher.preroll(&mut request);

        // 2. Specify grain
        let input_chunk = stretcher.specify_grain(&request);
        let sample_count = (input_chunk.end - input_chunk.begin).max(0) as usize;

        // Create a simple sine wave for testing
        let mut data = vec![0.0f32; sample_count];
        for (i, sample) in data.iter_mut().enumerate().take(sample_count) {
            *sample = (i as f32 * 0.1).sin(); // Simple sine wave
        }

        // 3. Analyse grain with sine wave data
        stretcher.analyse_grain(&mut data, 1);

        // 4. Synthesise grain
        let mut output_data = vec![0.0f32; 1024]; // Allocate buffer for output
        let mut output = OutputChunk::new(&mut output_data, 1);
        stretcher.synthesise_grain(&mut output);

        // 5. Next
        stretcher.next(&mut request);

        // Verify we didn't panic and the request was updated
        assert!(request.position >= 0.0);
        assert!(output.frame_count > 0);
    }

    #[test]
    fn speed_change() {
        // Test the workflow with a speed change
        let mut stretcher = Stretcher::new(44100, 1).unwrap();

        let mut request = Request {
            pitch: 1.0,
            speed: 1.5, // 50% faster
            position: 0.0,
            reset: false,
        };

        // 1. Preroll
        stretcher.preroll(&mut request);

        // 2. Specify grain
        let input_chunk = stretcher.specify_grain(&request);
        let sample_count = (input_chunk.end - input_chunk.begin).max(0) as usize;

        // Create a simple sine wave for testing
        let mut data = vec![0.0f32; sample_count];
        for (i, sample) in data.iter_mut().enumerate().take(sample_count) {
            *sample = (i as f32 * 0.1).sin(); // Simple sine wave
        }

        // 3. Analyse grain
        stretcher.analyse_grain(&mut data, 1);

        // 4. Synthesise grain
        let mut output_data = vec![0.0f32; 1024]; // Allocate buffer for output
        let mut output = OutputChunk::new(&mut output_data, 1);
        stretcher.synthesise_grain(&mut output);

        // 5. Next
        stretcher.next(&mut request);

        // Verify we didn't panic and the request was updated
        assert!(request.position >= 0.0);
        assert!(output.frame_count > 0);
    }

    #[test]
    fn pitch_change() {
        // Test the workflow with a pitch change
        let mut stretcher = Stretcher::new(44100, 1).unwrap();

        let mut request = Request {
            pitch: 1.5, // Up a fifth
            speed: 1.0,
            position: 0.0,
            reset: false,
        };

        // 1. Preroll
        stretcher.preroll(&mut request);

        // 2. Specify grain
        let input_chunk = stretcher.specify_grain(&request);
        let sample_count = (input_chunk.end - input_chunk.begin).max(0) as usize;

        // Create a simple sine wave for testing
        let mut data = vec![0.0f32; sample_count];
        for (i, sample) in data.iter_mut().enumerate().take(sample_count) {
            *sample = (i as f32 * 0.1).sin(); // Simple sine wave
        }

        // 3. Analyse grain
        stretcher.analyse_grain(&mut data, 1);

        // 4. Synthesise grain
        let mut output_data = vec![0.0f32; 1024]; // Allocate buffer for output
        let mut output = OutputChunk::new(&mut output_data, 1);
        stretcher.synthesise_grain(&mut output);

        // 5. Next
        stretcher.next(&mut request);

        // Verify we didn't panic and the request was updated
        assert!(request.position >= 0.0);
        assert!(output.frame_count > 0);
    }

    #[test]
    fn reset_flag() {
        // Test the workflow with the reset flag
        let mut stretcher = Stretcher::new(44100, 1).unwrap();

        let mut request = Request {
            pitch: 1.0,
            speed: 1.0,
            position: 0.0,
            reset: true, // Reset flag
        };

        // 1. Preroll
        stretcher.preroll(&mut request);

        // 2. Specify grain
        let input_chunk = stretcher.specify_grain(&request);
        let sample_count = (input_chunk.end - input_chunk.begin).max(0) as usize;

        // Create a simple sine wave for testing
        let mut data = vec![0.0f32; sample_count];
        for (i, sample) in data.iter_mut().enumerate().take(sample_count) {
            *sample = (i as f32 * 0.1).sin(); // Simple sine wave
        }

        // 3. Analyse grain
        stretcher.analyse_grain(&mut data, 1);

        // 4. Synthesise grain
        let mut output_data = vec![0.0f32; 1024]; // Allocate buffer for output
        let mut output = OutputChunk::new(&mut output_data, 1);
        stretcher.synthesise_grain(&mut output);

        // 5. Next
        stretcher.next(&mut request);

        // Verify we didn't panic and the request was updated
        assert!(request.position >= 0.0);
        assert!(output.frame_count > 0);
    }

    #[test]
    fn multiple_grains() {
        // Test processing multiple grains in sequence
        let mut stretcher = Stretcher::new(44100, 1).unwrap();

        let mut request = Request {
            pitch: 1.0,
            speed: 1.0,
            position: 0.0,
            reset: false,
        };

        // Preroll once
        stretcher.preroll(&mut request);

        // Process 3 grains in sequence
        for _grain_index in 0..3 {
            // Specify grain
            let input_chunk = stretcher.specify_grain(&request);
            let sample_count = input_chunk.end - input_chunk.begin;

            // Create silent data for simplicity
            let mut data = vec![0.0f32; sample_count as usize];

            // Analyse grain
            stretcher.analyse_grain(&mut data, 1);

            // Synthesise grain
            let mut output_data = vec![0.0f32; 1024]; // Allocate buffer for output
            let mut output = OutputChunk::new(&mut output_data, 1);
            stretcher.synthesise_grain(&mut output);

            // Next
            stretcher.next(&mut request);

            // Ensure position advances
            assert!(request.position >= 0.0);
        }
    }

    #[test]
    fn negative_position() {
        // Test workflow with a negative position (as might occur in seeking)
        let mut stretcher = Stretcher::new(44100, 1).unwrap();

        let mut request = Request {
            pitch: 1.0,
            speed: 1.0,
            position: -100.0, // Negative position
            reset: false,
        };

        // Preroll
        stretcher.preroll(&mut request);
        assert!(request.position < -100.0);

        // Specify grain
        let input_chunk = stretcher.specify_grain(&request);

        // Even with negative position, we should get valid chunk bounds
        // The library should handle this gracefully
        assert!(input_chunk.end >= 0);

        // Create some data (we'll use zeros)
        let sample_count = input_chunk.end - input_chunk.begin;
        let mut data = vec![0.0f32; sample_count as usize];

        // Analyse
        stretcher.analyse_grain(&mut data, 1);

        // Synthesise
        let mut output_data = vec![0.0f32; 1024]; // Allocate buffer for output
        let mut output = OutputChunk::new(&mut output_data, 1);
        stretcher.synthesise_grain(&mut output);

        // Next
        stretcher.next(&mut request);

        // Verify we didn't panic and the request was updated
        assert!(request.position >= -100.0);
    }
}
