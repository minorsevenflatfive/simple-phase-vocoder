use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound;
use std::f32::consts::PI;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read WAV file
    let mut reader = hound::WavReader::open("test.wav")?;
    let samples: Vec<f32> = reader.samples::<i16>()
    .map(|s| s.unwrap() as f32 / i16::MAX as f32)
    .collect();

    // Create a thread-safe buffer
    let samples = Arc::new(Mutex::new(VecDeque::from(samples)));

    // Get the default host
    let host = cpal::default_host();

    // Get all devices available
    let devices = host.devices()?;

    // Iterate over the devices and print their supported output configurations
    for (index, device) in devices.enumerate() {
        let name = device.name()?;
        println!("Device {}: {}", index + 1, name);

        match device.supported_output_configs() {
            Ok(configs) => {
                for (config_index, config) in configs.enumerate() {
                    println!("  Config {}: {:?}", config_index + 1, config);
                }
            },
            Err(e) => println!("  Error getting supported output configs: {:?}", e),
        }
    }

    // Get the default output device
    let output_device = host.default_output_device().ok_or("No output device available")?;

    // Get the default output configuration
    let config = output_device.default_output_config()?;

    // Build the output stream
    let samples_clone = Arc::clone(&samples);
    let sample_format = config.sample_format();
    println!("{}", sample_format);
    // let stream = output_device.build_output_stream(
    //     &config.into(),
    //     move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
    //         let mut samples = samples_clone.lock().unwrap();
    //         for sample in data.iter_mut() {
    //             *sample = match sample_format {
    //                 cpal::SampleFormat::F32 => samples.pop_front().unwrap_or(0.0) as f32 / i16::MAX as f32,
    //                 _ => 0.0,
    //             };
    //         }
    //     },
    //     |err| eprintln!("an error occurred on stream: {}", err),
    //     None,  // Using the default buffer size
    // )?;
    let stream = match sample_format {
        cpal::SampleFormat::F32 => output_device.build_output_stream(
            &config.into(), 
            move |data: &mut [f32], 
            |err| eprintln!("an error occured on stream: {}", err), 
            None,
        ),
        _ => return Err(cpal::BuildStreamError::StreamConfigNotSupported),
    }.unwrap();

    // Play the stream
    stream.play()?;

    // Keep the thread alive while the stream is playing
    loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        if samples.lock().unwrap().is_empty() {
            break;
        }
    }

    Ok(())
}

fn segment_into_frames(samples: &Vec<f32>, frame_size: usize, hop_size: usize) -> Vec<Vec<f32>> {
    let mut frames = Vec::new();
    let mut start = 0;
    while start + frame_size <= samples.len() {
        let mut frame = samples[start..start + frame_size].to_vec();
        apply_hanning_window(&mut frame);
        frames.push(frame);
        start += hop_size;
    }

    frames
}

fn apply_hanning_window(frame: &mut [f32]) {
    let frame_len = frame.len();
    for (i, sample) in frame.iter_mut().enumerate() {
        *sample *= 0.5 * (1.0 - (2.0 * PI * i as f32 / (frame_len as f32 - 1.0)).cos());
    }
}
