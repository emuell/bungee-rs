#![doc=include_str!("../README.md")]

use bungee_sys::BungeeStream;

use crate::Stretcher;

// -------------------------------------------------------------------------------------------------

/// A wrapper for `Stretcher` that provides an easy to use API for "streaming" applications
/// where Bungee is used for forward playback only.
pub struct Stream {
    #[allow(dead_code)]
    stretcher: Stretcher,
    stream: *mut BungeeStream,
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
            stretcher.inner(),
            num_channels as i32,
            max_input_frame_count as i32,
        );

        let input_pointers = vec![std::ptr::null(); num_channels];
        let output_pointers = vec![std::ptr::null_mut(); num_channels];

        Ok(Stream {
            stream,
            stretcher,
            input_pointers,
            output_pointers,
        })
    }

    /// Returns the stretcher's sample rate.  
    pub fn sample_rate(&self) -> usize {
        self.stretcher.sample_rate()
    }

    /// Returns the stretcher's channel layout.  
    pub fn num_channels(&self) -> usize {
        self.stretcher.num_channels()
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
        // verify input/output frame constraints
        assert!(
            input_frame_count > 0,
            "invalid input frame count: got {input_frame_count} frames, but need frames > 0"
        );
        assert!(
            output_frame_count > 0.0,
            "invalid output frame count: got {output_frame_count} frames, but need frames > 0"
        );

        // verify input data constraints
        if let Some(inputs) = input_channels {
            assert_eq!(
                inputs.len(),
                self.num_channels(),
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

        // verify output data constraints
        assert_eq!(
            output_channels.len(),
            self.num_channels(),
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
