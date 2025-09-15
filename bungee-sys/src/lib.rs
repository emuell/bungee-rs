//! Provides raw, low-level bindings to the Bungee C API.

use std::os::raw::{c_char, c_double, c_float, c_int};

/// The C API's `bool` type, which is defined as `char`.
pub type BungeeBool = u8;

/// Opaque handle to a Bungee stretcher implementation instance.
/// Corresponds to `void *implementation` in the C API.
#[repr(C)]
#[derive(Debug)]
pub struct BungeeStretcher {
    _private: [u8; 0],
}

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
    pub reset: BungeeBool,
}

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

/// Stretcher audio sample rates, in Hz.
#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SampleRates {
    pub input: c_int,
    pub output: c_int,
}

/// A struct of function pointers to access the functions of the Bungee stretcher.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct Functions {
    /// Reports, for example, "Pro" or "Basic".
    pub edition: Option<unsafe extern "C" fn() -> *const c_char>,

    /// Reports the release number of the library, for example "1.2.3".
    pub version: Option<unsafe extern "C" fn() -> *const c_char>,

    /// Initialises a stretcher instance.
    pub create: Option<
        unsafe extern "C" fn(
            sample_rates: SampleRates,
            channel_count: c_int,
            log2_synthesis_hop_adjust: c_int,
        ) -> *mut BungeeStretcher,
    >,

    /// Destroys a stretcher instance.
    pub destroy: Option<unsafe extern "C" fn(implementation: *mut BungeeStretcher)>,

    /// If called with a non-zero parameter, enables verbose diagnostics and checks.
    pub enable_instrumentation:
        Option<unsafe extern "C" fn(implementation: *mut BungeeStretcher, enable: c_int)>,

    /// Returns the largest number of frames that might be requested by `specifyGrain()`.
    pub max_input_frame_count:
        Option<unsafe extern "C" fn(implementation: *const BungeeStretcher) -> c_int>,

    /// Adjusts `request.position` for a run-in.
    pub preroll:
        Option<unsafe extern "C" fn(implementation: *const BungeeStretcher, request: *mut Request)>,

    /// Prepares `request.position` and `request.reset` for the subsequent grain.
    pub next:
        Option<unsafe extern "C" fn(implementation: *const BungeeStretcher, request: *mut Request)>,

    /// Specifies a grain and computes the necessary input audio segment.
    pub specify_grain: Option<
        unsafe extern "C" fn(
            implementation: *mut BungeeStretcher,
            request: *const Request,
            buffer_start_position: c_double,
        ) -> InputChunk,
    >,

    /// Begins processing the grain with the provided audio data.
    pub analyse_grain: Option<
        unsafe extern "C" fn(
            implementation: *mut BungeeStretcher,
            data: *const c_float,
            channel_stride: isize, // Corresponds to intptr_t
            mute_frame_count_head: c_int,
            mute_frame_count_tail: c_int,
        ),
    >,

    /// Completes processing of the grain and writes the output.
    pub synthesise_grain: Option<
        unsafe extern "C" fn(implementation: *mut BungeeStretcher, output_chunk: *mut OutputChunk),
    >,

    /// Returns true (non-zero) if the stretcher's pipeline is flushed.
    pub is_flushed:
        Option<unsafe extern "C" fn(implementation: *const BungeeStretcher) -> BungeeBool>,
}

#[allow(unused)]
extern "C" {
    /// Returns a pointer to the functions for the Bungee Basic edition.
    #[cfg(not(feature = "bungee_pro"))]
    pub fn getFunctionsBungeeBasic() -> *const Functions;

    /// Returns a pointer to the functions for the Bungee Pro edition.
    #[cfg(feature = "bungee_pro")]
    pub fn getFunctionsBungeePro() -> *const Functions;
}
