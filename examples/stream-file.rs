use std::error::Error;

use arg::{parse_args, Args};

use bungee_rs::Stream;

// -------------------------------------------------------------------------------------------------

#[derive(Args, Debug)]
struct Arguments {
    #[arg(short = "s", long = "speed", default_value = "1.0")]
    /// The stretch speed in range (0 - 100]
    speed: f64,
    #[arg(short = "p", long = "pitch", default_value = "1.0")]
    /// The resampling pitch in range (0 - 100]
    pitch: f64,
    /// Input file path
    input_path: String,
    /// Output file path
    output_path: String,
}

// -------------------------------------------------------------------------------------------------

#[allow(clippy::needless_range_loop)]
fn main() -> Result<(), Box<dyn Error>> {
    // Parse cmd arguments
    let args = parse_args::<Arguments>();

    let input_path: String = args.input_path;
    if input_path.is_empty() {
        return Err("Please specify an input path as first argument".into());
    };
    let output_path: String = args.output_path;
    if output_path.is_empty() {
        return Err("Please specify an output path as second argument".into());
    };
    let speed: f64 = args.speed;
    if !(0.0..=100.0).contains(&speed) {
        return Err("Invalid speed argument".into());
    }
    let pitch: f64 = args.pitch;
    if !(0.0..=100.0).contains(&pitch) {
        return Err("Invalid pitch argument".into());
    }

    println!("Converting `{input_path}` -> `{output_path}`");
    println!("Speed: {speed:.2}x, Pitch: {pitch:.2}x");

    // Open Wav input file
    let mut wav_reader = wavers::Wav::<f32>::from_path(&input_path)?;
    let num_channels = wav_reader.n_channels() as usize;
    let sample_rate = wav_reader.sample_rate() as usize;

    let mut wav_output_samples: Vec<i16> = Vec::new(); // interleaved
    let mut wav_write = {
        let output_samples = &mut wav_output_samples;
        move |planar_output: &Vec<Vec<f32>>, frame_offset: usize, frame_count: usize| {
            for frame in frame_offset..frame_count {
                for channel in 0..num_channels {
                    let sample = planar_output[channel][frame];
                    output_samples.push((sample * 32767.0) as i16);
                }
            }
        }
    };

    // Calculate needed block sizes
    let input_block_size: usize = 1024;
    let output_block_size = (input_block_size as f64 / speed).ceil() as usize;

    // Create stretcher stream
    let mut stream = Stream::new(sample_rate, num_channels, input_block_size)?;

    // Prepare temporary stream buffers (planar)
    let mut input_stream_buffer = vec![vec![0.0f32; input_block_size]; num_channels];
    let mut output_stream_buffer = vec![vec![0.0f32; output_block_size]; num_channels];

    // Process
    let mut wav_reader_frames = wav_reader.frames();
    let mut remaining_latency_frames = None;

    loop {
        // read interleaved frames into planar input buffer
        let mut frames_read = 0;
        for frame in 0..input_block_size {
            if let Some(samples) = wav_reader_frames.next() {
                for channel in 0..num_channels {
                    input_stream_buffer[channel][frame] = samples[channel];
                }
                frames_read += 1;
            } else {
                break;
            }
        }

        if frames_read > 0 {
            // process
            let input_frames = frames_read;
            let output_frames_f = frames_read as f64 / speed;
            let processed_frames = stream.process(
                Some(&input_stream_buffer),
                &mut output_stream_buffer,
                input_frames,
                output_frames_f,
                pitch,
            );

            // fetch stream's latency after the first process call
            let remaining_frames_to_skip = *remaining_latency_frames
                .get_or_insert_with(|| (stream.latency() as f64 / speed) as usize);

            // skip empty latency buffers with the first process calls
            let frames_to_skip = remaining_frames_to_skip.min(processed_frames);
            remaining_latency_frames.replace(remaining_frames_to_skip - frames_to_skip);

            // write result to interleaved sample buffer vector
            wav_write(&output_stream_buffer, frames_to_skip, processed_frames);
        }

        if frames_read < input_block_size {
            break; // End of input file
        }
    }

    // flush remaining output samples
    let mut remaining_input_frames = stream.latency().ceil() as usize;
    loop {
        // process with mute input
        let input_frames = remaining_input_frames.min(input_block_size);
        let output_frames_f = input_frames as f64 / speed;
        let processed_frames = stream.process(
            None,
            &mut output_stream_buffer,
            input_frames,
            output_frames_f,
            pitch,
        );

        // write result to interleaved sample buffer vector
        wav_write(&output_stream_buffer, 0, processed_frames);

        remaining_input_frames = remaining_input_frames.saturating_sub(input_frames);
        if remaining_input_frames == 0 {
            break; // End of output file
        }
    }

    // Write Wav output file
    wavers::write(
        &output_path,
        &wav_output_samples,
        sample_rate as i32,
        num_channels as u16,
    )?;

    println!("Done.");

    Ok(())
}
