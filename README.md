# bungee-rs: Rust FFI Bindings for Bungee Audio Time-Stretching Library

`bungee-rs` provides safe Rust bindings for the [Bungee](https://github.com/bungee-audio-stretch/bungee) C++ audio time-stretching and pitch-shifting library. 

See also `bungee-sys` for a raw, low-level FFI wrapper for Bungee.

## Installation

Add `bungee-rs` as a dependency in your `Cargo.toml`:

```toml
[dependencies]
bungee-rs = { path = "PATH_TO/bungee-rs" }  # TODO publish to crates.io
```

## Building

### Prerequisites

- Rust toolchain (stable)
- C++17 compatible compiler (msvc, clang++ or g++)
- CMake (for building Bungee dependencies)

### Build Steps

1. Clone the repository **with submodules**:
   ```bash
   git clone --recurse-submodules https://github.com/emuell/bungee-rs.git
   ```

2. Navigate to the `bungee-rs` directory and build the crate:
   ```bash
   cd bungee-rs && cargo build
   ```

## Usage Example

```rust
use bungee_rs::{Request, Stretcher, OutputChunk};

fn main() -> Result<(), &'static str> {
    // Create a stereo stretcher at 44.1kHz
    let mut stretcher = Stretcher::new(44100, 2)?;
    
    // Configure playback parameters
    let mut request = Request {
        pitch: 1.0,    // No pitch shift
        speed: 0.75,   // 75% playback speed
        position: 0.0, // Start at beginning
    };
    
    // Preroll to initialize internal state
    stretcher.preroll(&request);
    
    // Process audio in grains
    loop {
        // Get required input samples
        let input_chunk = stretcher.specify_grain(&request);
        
        // Provide input audio data (your implementation here)
        // stretcher.analyse_grain(input_data, channel_stride);
        
        // Get output audio
        let mut data = vec![f32; 1024];
        let mut output_chunk = OutputChunk::new(data, 0);
        stretcher.synthesise_grain(&mut output_chunk);
        
        // Use output audio (your implementation here)
        
        // Advance to next grain
        stretcher.next(&mut request);
    }
    
    Ok(())
}
```

## License

`bungee-rs` is licensed under the MPL-2.0 license, consistent with the upstream Bungee library.
