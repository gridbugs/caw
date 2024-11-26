use caw_core::signal::{Sf64, Signal};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    InputCallbackInfo, StreamConfig,
};
use std::{cell::Cell, sync::mpsc};

pub fn microphone() -> anyhow::Result<Sf64> {
    let host = cpal::default_host();
    log::info!("cpal host: {}", host.id().name());
    let device = host
        .default_input_device()
        .ok_or(anyhow::anyhow!("no input device"))?;
    if let Ok(name) = device.name() {
        log::info!("cpal device: {}", name);
    } else {
        log::info!("cpal device: (no name)");
    }
    let config = device.default_input_config()?;
    log::info!("sample format: {}", config.sample_format());
    log::info!("sample rate: {}", config.sample_rate().0);
    log::info!("num channels: {}", config.channels());
    let config = StreamConfig::from(config);
    let channels = config.channels;
    let (sender, receiver) = mpsc::channel::<f32>();
    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _info: &InputCallbackInfo| {
                for channel_data in data.chunks(channels as usize) {
                    let mean_sample =
                        channel_data.iter().sum::<f32>() / channels as f32;
                    if sender.send(mean_sample).is_err() {
                        log::error!("failed to send data from cpal thread");
                    }
                }
            },
            |err| log::error!("stream error: {}", err),
            None,
        )
        .unwrap();
    stream.play()?;
    let sample_rate_hz = config.sample_rate.0 as f64;
    let warned_about_mismatched_sample_rate = Cell::new(false);
    // Store the previous sample so it can be used if there is no new
    // data available.
    let prev_sample = Cell::new(0.0);
    let first = Cell::new(true);
    Ok(Signal::from_fn(move |ctx| {
        // Capture the stream in the returned closure so that it
        // doesn't get dropped at the end of this function.
        let _force_closure_to_own_stream = &stream;
        if sample_rate_hz != ctx.sample_rate_hz
            && !warned_about_mismatched_sample_rate.get()
        {
            warned_about_mismatched_sample_rate.set(true);
            log::warn!(
                "input sample rate of {}Hz doesn not match output sample rate of {}Hz",
                sample_rate_hz,
                ctx.sample_rate_hz
            );
        }
        let mut latest_sample = None;
        if first.get() {
            first.set(false);
            // Catch up to the sender on the first sample to remove
            // any delay between creating the microphone and starting
            // sampling.
            while let Ok(sample) = receiver.try_recv() {
                latest_sample = Some(sample);
            }
        } else if let Ok(sample) = receiver.try_recv() {
            latest_sample = Some(sample);
        }
        if let Some(sample) = latest_sample {
            prev_sample.set(sample);
            sample as f64
        } else {
            prev_sample.get() as f64
        }
    }))
}
