//! Provides raw, low-level bindings to the Bungee C++ API.
#![recursion_limit = "512"]

use std::ffi::{c_double, c_float, c_int, c_uchar};

// -------------------------------------------------------------------------------------------------

/// Opaque handle to a Bungee stretcher implementation instance.
/// Corresponds to `void *implementation` in the C API.
#[repr(C)]
#[derive(Debug)]
pub struct BungeeStretcher {
    _private: [u8; 0],
}

// -------------------------------------------------------------------------------------------------

/// Opaque handle to a Bungee stream implementation instance.
#[repr(C)]
#[derive(Debug)]
pub struct BungeeStream {
    _private: [u8; 0],
}

// -------------------------------------------------------------------------------------------------

/// An object of type `Request` is passed to the audio stretcher every time an audio grain is processed.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Request {
    /// Frame-offset within the input audio of the centre-point of the current audio grain.
    /// `NaN` signifies an invalid grain that produces no audio output and may be used for flushing.
    pub position: c_double,

    /// Output audio speed. A value of 1.0 means speed should be unchanged relative to the input audio.
    /// Used by Stretcher's internal algorithms only when it's not possible to determine speed by
    /// subtracting `Request::position` of the previous grain from the current grain.
    pub speed: c_double,

    /// Adjustment as a frequency multiplier with a value of 1.0 meaning no pitch adjustment.
    pub pitch: c_double,

    /// Set to have the stretcher forget all previous grains and restart on this grain.
    /// (0 for false, non-zero for true)
    pub reset: c_uchar,
}

// -------------------------------------------------------------------------------------------------

/// Information to describe a chunk of audio that the audio stretcher requires as input for the current grain.
/// Note that input chunks of consecutive grains often overlap and are usually centred on the grain's
/// `Request::position`.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct InputChunk {
    /// Frame offsets relative to the start of the audio track.
    pub begin: c_int,
    pub end: c_int,
}

// -------------------------------------------------------------------------------------------------

/// Describes a chunk of audio output.
/// Output chunks do not overlap and can be appended for seamless playback.
#[repr(C)]
#[derive(Debug)]
pub struct OutputChunk {
    /// Audio output data, not aligned and not interleaved.
    pub data: *mut c_float,
    /// Number of frames in the output data.
    pub frame_count: c_int,
    /// The nth audio channel audio starts at `data[n * channelStride]`.
    pub channel_stride: isize, // Corresponds to intptr_t
    /// `request[0]` corresponds to the first frame of data, `request[1]` corresponds to the frame
    /// after the last frame of data.
    pub request: [*const Request; 2],
}

// -------------------------------------------------------------------------------------------------

/// Stretcher audio sample rates, in Hz.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SampleRates {
    pub input: c_int,
    pub output: c_int,
}

// -------------------------------------------------------------------------------------------------

pub mod stream;
pub mod stretcher;
