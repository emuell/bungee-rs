# High-level Rust FFI Bindings for Bungee Audio Time-Stretching Library

`bungee-rs` provides safe Rust bindings for the [Bungee](https://github.com/bungee-audio-stretch/bungee) C++ audio time-stretching and pitch-shifting library. 

See also [`bungee-sys`](./bungee-sys/README.md) for a raw, low-level FFI wrapper for Bungee.

## Usage

There are two ways to use `bungee-rs`: 
* a high-level `Stream` API for simple, forward-only playback
* a low-level, grain-by-grain `Stretcher` API for other more complex scenarios.

### High-Level Stream API

This is the recommended API for most use cases, such as real-time audio processing.
See also [examples/stream-file.rs](./examples/stream-file.rs)

```rust, no_run
use bungee_rs::{Stretcher, Stream};

fn main() -> Result<(), &'static str> {
    // Test setup: For 0.75x speed, output is larger than input.
    const SAMPLE_RATE: usize = 44100;
    const NUM_CHANNELS: usize = 2;
    const STRETCH_FACTOR: f64 = 0.75;
    const INPUT_BLOCK_SIZE: usize = 1024;
    const OUTPUT_BLOCK_SIZE: f64 = INPUT_BLOCK_SIZE as f64 / STRETCH_FACTOR;

    // Create a stereo stretcher stream
    let mut stream = Stream::new(SAMPLE_RATE, NUM_CHANNELS, INPUT_BLOCK_SIZE)?;

    // Prepare planar input and output buffers.
    let input_block = vec![vec![0.0f32; INPUT_BLOCK_SIZE]; NUM_CHANNELS];
    let mut output_block = vec![vec![0.0f32; OUTPUT_BLOCK_SIZE.ceil() as usize]; NUM_CHANNELS];

    // In a real application, you would loop here, reading audio into `input_block`.

    // Process one block of audio
    let output_frame_count = stream.process(
        Some(&input_block),
        &mut output_block,
        INPUT_BLOCK_SIZE,
        OUTPUT_BLOCK_SIZE,
        1.0, // No pitch shift
    );

    // `output_frame_count` now contains the number of valid frames in `output_block`.

    Ok(())
}
```

### Low-Level Stretcher API

This API gives you fine-grained control over the stretching process, which is useful for non-linear access or custom processing loops, but requires access to the entire audio input data.

```rust, no_run
use bungee_rs::{Request, Stretcher, OutputChunk};

fn main() -> Result<(), &'static str> {
    // Create a stereo stretcher at 44.1kHz
    let mut stretcher = Stretcher::new(44100, 2)?;
    
    // Configure playback parameters
    let mut request = Request {
        pitch: 1.0,    // No pitch shift
        speed: 0.75,   // 75% playback speed
        position: 0.0, // Start at beginning
        reset: false,
    };
    
    // Preroll to initialize internal state
    stretcher.preroll(&mut request);
    
    // A real implementation would have a loop here that continues as long as there
    // is audio to process. For this example, we'll just process one grain.
    
    // Get required input samples
    let input_chunk = stretcher.specify_grain(&request);
    
    // Provide input audio data (add your implementation here).
    // Bungee expects non-interleaved audio. For this example, we'll use silence.
    let num_frames = input_chunk.len();
    let num_channels = 2;
    let mut input_data = vec![0.0f32; num_frames * num_channels];
    // The channel stride for non-interleaved data is the number of frames per channel.
    stretcher.analyse_grain(&mut input_data, num_frames);
    
    // Get output audio
    // The output data is also non-interleaved.
    let max_output_frames = 2048;
    let mut output_data = vec![0.0f32; max_output_frames * num_channels];
    let mut output_chunk = OutputChunk::new(&mut output_data, max_output_frames);
    stretcher.synthesise_grain(&mut output_chunk);
    
    // Use the output audio. `output_chunk.frame_count` will have the number of valid frames.
    
    // Advance to next grain for the next iteration of the loop
    stretcher.next(&mut request);
    
    Ok(())
}
```

## License

`bungee-rs` is licensed under the MPL-2.0 license, consistent with the upstream Bungee C++ library.
