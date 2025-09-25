//! FFI bindings to the Bungee C++ Stream class.
#[allow(non_upper_case_globals)]
use std::ffi::{c_double, c_float, c_int};

use crate::{BungeeStream, BungeeStretcher};
use cpp::cpp;

// -------------------------------------------------------------------------------------------------

cpp! {{
    #include "Stream.h"
    
    using namespace Bungee;
    using Edition = Bungee::Basic;
}}

/// Creates a stream processor.
pub fn create(
    stretcher: *mut BungeeStretcher,
    num_channels: c_int,
    max_input_frame_count: c_int,
) -> *mut BungeeStream {
    unsafe {
        cpp!([
            stretcher as "Bungee::Stretcher<Edition> *",
            num_channels as "int",
            max_input_frame_count as "int"
        ] -> *mut BungeeStream as "void *" {
            return (void *)new Bungee::Stream<Edition>(*stretcher, max_input_frame_count, num_channels);
        })
    }
}

/// Destroys a stream processor.
pub fn destroy(stream: *mut BungeeStream) {
    unsafe {
        cpp!([stream as "Bungee::Stream<Edition> *"] {
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
