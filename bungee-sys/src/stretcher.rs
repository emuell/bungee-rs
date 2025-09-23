//! Provides raw, low-level bindings to the Bungee C++ Stretcher class.

use std::ffi::{c_char, c_double, c_float, c_int, c_uchar};

use cpp::cpp;

use crate::{BungeeStretcher, InputChunk, OutputChunk, Request, SampleRates};

// -------------------------------------------------------------------------------------------------

cpp! {{
    #include "Bungee.h"
    #include <cstdint>

    using namespace Bungee;
    using Edition = Bungee::Basic;
}}

/// Reports, for example, "Pro" or "Basic".
pub fn edition() -> *const c_char {
    unsafe {
        cpp!([] -> *const c_char as "const char *" {
            return Stretcher<Edition>::edition();
        })
    }
}

/// Reports the release number of the library, for example "1.2.3".
pub fn version() -> *const c_char {
    unsafe {
        cpp!([] -> *const c_char as "const char *" {
            return Stretcher<Edition>::version();
        })
    }
}

/// Initialises a stretcher instance.
pub fn create(
    sample_rates: SampleRates,
    channel_count: c_int,
    log2_synthesis_hop_adjust: c_int,
) -> *mut BungeeStretcher {
    unsafe {
        cpp!([
            sample_rates as "SampleRates",
            channel_count as "int",
            log2_synthesis_hop_adjust as "int"
        ] -> *mut BungeeStretcher as "void *" {
            return (void *)new Stretcher<Edition>(sample_rates, channel_count, log2_synthesis_hop_adjust);
        })
    }
}

/// Destroys a stretcher instance.
pub fn destroy(stretcher: *mut BungeeStretcher) {
    unsafe {
        cpp!([stretcher as "Stretcher<Edition> *"] {
            delete stretcher;
        })
    }
}

/// If called with a non-zero parameter, enables verbose diagnostics and checks.
pub fn enable_instrumentation(stretcher: *mut BungeeStretcher, enable: c_int) {
    unsafe {
        cpp!([stretcher as "Stretcher<Edition> *", enable as "int"] {
            stretcher->enableInstrumentation(enable);
        })
    }
}

/// Returns the largest number of frames that might be requested by `specifyGrain()`.
pub fn max_input_frame_count(stretcher: *const BungeeStretcher) -> c_int {
    unsafe {
        cpp!([stretcher as "const Stretcher<Edition> *"] -> c_int as "int" {
            return stretcher->maxInputFrameCount();
        })
    }
}

/// Adjusts `request.position` for a run-in.
pub fn preroll(stretcher: *const BungeeStretcher, request: *mut Request) {
    unsafe {
        cpp!([stretcher as "const Stretcher<Edition> *", request as "Request *"] {
            stretcher->preroll(*request);
        })
    }
}

/// Prepares `request.position` and `request.reset` for the subsequent grain.
pub fn next(stretcher: *const BungeeStretcher, request: *mut Request) {
    unsafe {
        cpp!([stretcher as "const Stretcher<Edition> *", request as "Request *"] {
            stretcher->next(*request);
        })
    }
}

/// Specifies a grain and computes the necessary input audio segment.
pub fn specify_grain(
    stretcher: *mut BungeeStretcher,
    request: *const Request,
    buffer_start_position: c_double,
) -> InputChunk {
    unsafe {
        cpp!([
            stretcher as "Stretcher<Edition> *",
            request as "const Request *",
            buffer_start_position as "double"
        ] -> InputChunk as "InputChunk" {
            return stretcher->specifyGrain(*request, buffer_start_position);
        })
    }
}

/// Begins processing the grain with the provided audio data.
pub fn analyse_grain(
    stretcher: *mut BungeeStretcher,
    data: *const c_float,
    channel_stride: isize,
    mute_frame_count_head: c_int,
    mute_frame_count_tail: c_int,
) {
    unsafe {
        cpp!([
            stretcher as "Stretcher<Edition> *",
            data as "const float *",
            channel_stride as "intptr_t",
            mute_frame_count_head as "int",
            mute_frame_count_tail as "int"
        ] {
            stretcher->analyseGrain(data, channel_stride, mute_frame_count_head, mute_frame_count_tail);
        })
    }
}

/// Completes processing of the grain and writes the output.
pub fn synthesise_grain(stretcher: *mut BungeeStretcher, output_chunk: *mut OutputChunk) {
    unsafe {
        cpp!([
            stretcher as "Stretcher<Edition> *",
            output_chunk as "OutputChunk *"
        ] {
            stretcher->synthesiseGrain(*output_chunk);
        })
    }
}

/// Returns true (non-zero) if the stretcher's pipeline is flushed.
pub fn is_flushed(stretcher: *const BungeeStretcher) -> c_uchar {
    unsafe {
        cpp!([stretcher as "const Stretcher<Edition> *"] -> c_uchar as "bool" {
            return stretcher->isFlushed();
        })
    }
}
