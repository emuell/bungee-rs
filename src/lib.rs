#![doc=include_str!("../README.md")]

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

mod stream;
pub use stream::Stream;

mod stretcher;
pub use stretcher::Stretcher;
