#![doc=include_str!("../README.md")]

use bungee_sys::{BungeeStream, BungeeStretcher};

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

impl InputChunk {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn len(&self) -> usize {
        (self.end - self.begin).max(0) as usize
    }
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

    pub fn sample_rate(&self) -> usize {
        self.sample_rate
    }

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
    /// by a prior call to `bungee_specify_grain`.
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
    ///
    /// # Safety
    /// `output` must be a valid pointer to an `OutputChunk` struct that the library can write into.
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
}

impl Drop for Stretcher {
    /// Destroys a Bungee stretcher instance and frees its memory.
    fn drop(&mut self) {
        bungee_sys::stretcher::destroy(self.inner);
    }
}

// -------------------------------------------------------------------------------------------------

/// A wrapper for `Stretcher` that provides an easy to use API for "streaming" applications
/// where Bungee is used for forward playback only.
pub struct Stream {
    #[allow(dead_code)]
    stretcher: Stretcher,
    stream: *mut BungeeStream,
    channel_count: usize,
    input_pointers: Vec<*const f32>,
    output_pointers: Vec<*mut f32>,
}

unsafe impl Send for Stream {}
unsafe impl Sync for Stream {}

impl Stream {
    /// Creates a new `Stream` instance from a stretcher instance.
    pub fn new(
        sample_rate: usize,
        num_channels: usize,
        max_input_frame_count: usize,
    ) -> Result<Self, &'static str> {
        let stretcher = Stretcher::new(sample_rate, num_channels)?;

        let stream = bungee_sys::stream::create(
            stretcher.inner,
            num_channels as i32,
            max_input_frame_count as i32,
        );

        let channel_count = num_channels;

        let input_pointers = vec![std::ptr::null(); num_channels];
        let output_pointers = vec![std::ptr::null_mut(); num_channels];

        Ok(Stream {
            stream,
            stretcher,
            channel_count,
            input_pointers,
            output_pointers,
        })
    }

    /// Processes a segment of audio. Returns the number of output frames that were rendered
    /// to `output_channels`.
    /// The number of frames will be set by dithering either to `floor(output_frame_count)` or
    /// `ceil(output_frame_count)`.
    ///
    /// Parameters:
    /// * **input_channels:** Slice of `Vec<f32>`, one for each channel of input audio:
    ///   set to `None` for mute input
    /// * **output_channels:** Slice of `Vec<f32>`, one for each channel of output audio
    /// * **input_frame_count:** Number of input audio frames to be processed
    /// * **pitch:** Audio pitch shift (see Request::pitch)
    pub fn process(
        &mut self,
        input_channels: Option<&[Vec<f32>]>,
        output_channels: &mut [Vec<f32>],
        input_frame_count: usize,
        output_frame_count: f64,
        pitch: f64,
    ) -> usize {
        // verify input buffer constraints
        if let Some(inputs) = input_channels {
            assert_eq!(
                inputs.len(),
                self.channel_count,
                "input_channels slice count must match stream channel count"
            );
            for (channel, samples) in inputs.iter().enumerate() {
                assert!(
                    samples.len() >= input_frame_count,
                    "input channel[{}].len() ({}) is less than input_frame_count ({})",
                    channel,
                    samples.len(),
                    input_frame_count
                );
            }
        }
        if let Some(input_channels) = input_channels {
            for (p, c) in self.input_pointers.iter_mut().zip(input_channels) {
                *p = c.as_ptr();
            }
        } else {
            self.input_pointers.fill(std::ptr::null());
        }

        // verify output buffer constraints
        assert_eq!(
            output_channels.len(),
            self.channel_count,
            "output_channels slice count must match stream channel count"
        );
        let required_output_len = output_frame_count.ceil() as usize;
        for (channel, samples) in output_channels.iter().enumerate() {
            assert!(
                samples.len() >= required_output_len,
                "output channel[{}].len() ({}) is less than required output frame count ({})",
                channel,
                samples.len(),
                required_output_len
            );
        }
        for (p, c) in self.output_pointers.iter_mut().zip(output_channels) {
            *p = c.as_mut_ptr();
        }

        // process
        bungee_sys::stream::process(
            self.stream,
            if input_channels.is_none() {
                std::ptr::null()
            } else {
                self.input_pointers.as_ptr()
            },
            self.output_pointers.as_mut_ptr(),
            input_frame_count as i32,
            output_frame_count,
            pitch,
        ) as usize
    }

    /// Current position in the input stream. This is sum of `input_sample_count` over all `process()` calls.
    pub fn input_position(&self) -> isize {
        bungee_sys::stream::input_position(self.stream) as isize
    }

    /// Current position of the output stream in terms of input frames.
    pub fn output_position(&self) -> f64 {
        bungee_sys::stream::output_position(self.stream)
    }

    /// Latency due to the stretcher. Units are input frames.
    pub fn latency(&self) -> f64 {
        bungee_sys::stream::latency(self.stream)
    }
}

impl Drop for Stream {
    fn drop(&mut self) {
        bungee_sys::stream::destroy(self.stream);
    }
}

// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processing() {
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

    #[test]
    fn stream_processing() {
        const SAMPLE_RATE: usize = 44100;
        const NUM_CHANNELS: usize = 1;
        const STRETCH_FACTOR: f64 = 0.7;
        const INPUT_SAMPLES_COUNT: usize = 1024;
        const OUTPUT_SAMPLES_COUNT: f64 = INPUT_SAMPLES_COUNT as f64 / STRETCH_FACTOR;

        // max_input_sample_count for Stream::new is the max number of samples in one process() call.
        let mut stream = Stream::new(SAMPLE_RATE, NUM_CHANNELS, INPUT_SAMPLES_COUNT).unwrap();

        // Create a silent input buffer
        let input_channels = vec![vec![0.0f32; INPUT_SAMPLES_COUNT]];

        // Create an output buffer, make it a bit larger to be safe.
        let mut output_channels = vec![vec![0.0f32; OUTPUT_SAMPLES_COUNT.ceil() as usize * 2]];

        // Process one block of audio
        let output_sample_count = stream.process(
            Some(&input_channels),
            &mut output_channels,
            INPUT_SAMPLES_COUNT,
            OUTPUT_SAMPLES_COUNT,
            1.0,
        );

        // Output count should match
        assert_eq!(output_sample_count, OUTPUT_SAMPLES_COUNT.ceil() as usize);

        // Check stream state after processing.
        assert_eq!(stream.input_position(), INPUT_SAMPLES_COUNT as isize);
        assert!(stream.latency() > 0.0);
    }
}
