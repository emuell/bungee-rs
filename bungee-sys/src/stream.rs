use std::ffi::{c_double, c_float, c_int};

use crate::SampleRates;
use cpp::cpp;

// -------------------------------------------------------------------------------------------------

cpp! {{
    #include "Bungee.h"
    #include "Stream.h"
    #include <cstdint>
    #include <cmath>

    using namespace Bungee;
    using Edition = Bungee::Basic;
}}

/// Opaque handle to a Bungee stream implementation instance.
#[repr(C)]
#[derive(Debug)]
pub struct BungeeStream {
    _private: [u8; 0],
}

/// Creates a stream processor.
pub fn create(
    sample_rates: SampleRates,
    channel_count: c_int,
    max_input_frame_count: c_int,
    log2_synthesis_hop_adjust: c_int,
) -> *mut BungeeStream {
    unsafe {
        cpp!([
            sample_rates as "SampleRates",
            channel_count as "int",
            max_input_frame_count as "int",
            log2_synthesis_hop_adjust as "int"
        ] -> *mut BungeeStream as "void *" {
            auto* stretcher = new Bungee::Stretcher<Edition>(sample_rates, channel_count, log2_synthesis_hop_adjust);
            return (void *)new Bungee::Stream<Edition>(*stretcher, max_input_frame_count, channel_count);
        })
    }
}

/// Destroys a stream processor.
pub fn destroy(stream: *mut BungeeStream) {
    unsafe {
        cpp!([stream as "Bungee::Stream<Edition> *"] {
            delete &stream->stretcher();
            delete stream;
        })
    }
}

/// Processes a segment of audio.
pub fn process(
    stream: *mut BungeeStream,
    input_pointers: *const *const c_float,
    output_pointers: *mut *mut c_float,
    input_sample_count: c_int,
    output_sample_count: c_double,
    pitch: c_double,
) -> c_int {
    unsafe {
        cpp!([
            stream as "Bungee::Stream<Edition> *",
            input_pointers as "const float* const *",
            output_pointers as "float *const *",
            input_sample_count as "int",
            output_sample_count as "double",
            pitch as "double"
        ] -> c_int as "int" {
            return stream->process(
                input_pointers, output_pointers, input_sample_count, output_sample_count, pitch);
        })
    }
}

/// Current position of the output stream in terms of input samples.
pub fn input_position(stream: *const BungeeStream) -> c_int {
    unsafe {
        cpp!([stream as "const Bungee::Stream<Edition> *"] -> c_int as "int" {
            return stream->inputPosition();
        })
    }
}

/// Current position of the output stream in terms of output samples.
pub fn output_position(stream: *const BungeeStream) -> c_double {
    unsafe {
        cpp!([stream as "const Bungee::Stream<Edition> *"] -> c_double as "double" {
            return stream->outputPosition();
        })
    }
}

/// Current latency of the stream processor.
pub fn latency(stream: *const BungeeStream) -> c_double {
    unsafe {
        cpp!([stream as "const Bungee::Stream<Edition> *"] -> c_double as "double" {
            return stream->latency();
        })
    }
}
