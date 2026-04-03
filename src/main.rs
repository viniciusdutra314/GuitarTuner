use cpal::{
    StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("Not output device found")?;
    let mut supported_configs_range = device.supported_output_configs()?;
    let config: StreamConfig = supported_configs_range
        .next()
        .ok_or("")?
        .with_max_sample_rate()
        .into();
    let mut time = 0.0f32;
    let pi = std::f32::consts::PI;
    let sample_rate = config.sample_rate as f32;
    let frequency = 440.0;
    let channels = config.channels as usize;
    let stream = device.build_output_stream(
        &config.into(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                let value = f32::sin(2.0 * pi * (frequency / sample_rate) * time);

                for sample in frame.iter_mut() {
                    *sample = value;
                }
                time += 1 as f32;
            }
        },
        move |err| {},
        None,
    )?;

    stream.play()?;
    println!("Playing");
    std::thread::sleep(std::time::Duration::new(5, 0));
    return Ok(());
}
