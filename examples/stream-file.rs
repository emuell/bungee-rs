use std::error::Error;
use wavers::{write, Wav};

use arg::{parse_args, Args};

use bungee_rs::Stream;

// -------------------------------------------------------------------------------------------------

#[derive(Args, Debug)]
struct Arguments {
    #[arg(short = "s", long = "speed", default_value = "1.0")]
    /// The stretch speed in range (0 - 100]
    speed: f64,
    #[arg(short = "p", long = "pitch", default_value = "1.0")]
    /// The resampling pitch
    pitch: f64,
    /// Input and output file paths
    paths: Vec<String>,
}

// -------------------------------------------------------------------------------------------------

#[allow(clippy::needless_range_loop)]
fn main() -> Result<(), Box<dyn Error>> {
    // Parse cmd arguments
    let args = parse_args::<Arguments>();
    let input_path: String = args
        .paths
        .first()
        .expect("Please specify an input and output path as argument")
        .to_string();
    let output_path: String = args
        .paths
        .get(1)
        .expect("Please specify an input and output path as argument")
        .to_string();
    let speed: f64 = args.speed;
    let pitch: f64 = args.pitch;

    // Calculate needed block sizes
    let input_block_size: usize = 1024;
    let output_block_size_f = input_block_size as f64 / speed;
    let output_block_size = output_block_size_f.ceil() as usize;

    println!("Converting `{input_path}` -> `{output_path}`");
    println!("Speed: {speed:.2}x, Pitch: {pitch:.2}x");

    // Open Wav input file
    let mut reader = Wav::<f32>::from_path(&input_path)?;
    let mut wav_output_samples: Vec<i16> = Vec::new(); // interleaved
    let num_channels = reader.n_channels() as usize;
    let sample_rate = reader.sample_rate() as usize;

    // Prepare stretcher and stream
    let mut stream = Stream::new(sample_rate, num_channels, input_block_size)?;

    // Prepare temporary stream buffers
    let mut input_deinterleaved = vec![vec![0.0f32; input_block_size]; num_channels];
    let mut output_deinterleaved = vec![vec![0.0f32; output_block_size]; num_channels];

    // Process
    let mut frames_iter = reader.frames();
    let mut latency_frames = None;
    let mut total_frames_read = 0;

    loop {
        // read interleaved frames into planar input buffer
        let mut frames_read = 0;
        for i in 0..input_block_size {
            if let Some(frame) = frames_iter.next() {
                for ch in 0..num_channels {
                    input_deinterleaved[ch][i] = frame[ch];
                }
                frames_read += 1;
            } else {
                break;
            }
        }

        if frames_read > 0 {
            // process
            let output_expected = frames_read as f64 / speed;
            let output_frame_count = stream.process(
                Some(&input_deinterleaved),
                &mut output_deinterleaved,
                frames_read,
                output_expected,
                pitch,
            );

            // fetch stream's latency after the first process call
            let pending_latency_frames =
                *latency_frames.get_or_insert_with(|| (stream.latency() as f64 / speed) as usize);

            // skip empty latency buffers with the first process calls
            let frames_to_skip = pending_latency_frames.min(output_frame_count);
            latency_frames.replace(pending_latency_frames - frames_to_skip);

            // push result to interleaved i16 sample buffer
            for i in frames_to_skip..output_frame_count {
                for ch in 0..num_channels {
                    let sample = output_deinterleaved[ch][i];
                    wav_output_samples.push((sample * 32767.0) as i16);
                }
            }
        }

        total_frames_read += frames_read;

        if frames_read < input_block_size {
            break; // End of input file
        }
    }

    // flush remaining output samples
    loop {
        let output_expected = output_block_size_f;
        let output_frame_count = stream.process(
            None,
            &mut output_deinterleaved,
            input_block_size,
            output_expected,
            pitch,
        );

        let remaining_frames = total_frames_read.saturating_sub(stream.output_position() as usize);
        if remaining_frames == 0 {
            break;
        }

        for i in 0..output_frame_count.min(remaining_frames) {
            for ch in 0..num_channels {
                let sample = output_deinterleaved[ch][i];
                wav_output_samples.push((sample * 32767.0) as i16);
            }
        }
    }

    // Write Wav output file
    write(
        &output_path,
        &wav_output_samples,
        sample_rate as i32,
        num_channels as u16,
    )?;

    println!("Done.");

    Ok(())
}
