use caw_core_next::{Signal, SignalCtx};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, OutputCallbackInfo, StreamConfig,
};
use std::sync::{mpsc, Arc, RwLock};

pub trait ToF32 {
    fn to_f32(self) -> f32;
}

impl ToF32 for f32 {
    fn to_f32(self) -> f32 {
        self
    }
}

impl ToF32 for f64 {
    fn to_f32(self) -> f32 {
        self as f32
    }
}

pub struct Player {
    device: Device,
    config: StreamConfig,
}

impl Player {
    pub fn new() -> anyhow::Result<Self> {
        let host = cpal::default_host();
        log::info!("cpal host: {}", host.id().name());
        let device = host
            .default_output_device()
            .ok_or(anyhow::anyhow!("no output device"))?;
        if let Ok(name) = device.name() {
            log::info!("cpal device: {}", name);
        } else {
            log::info!("cpal device: (no name)");
        }
        let config = device.default_output_config()?;
        log::info!("sample format: {}", config.sample_format());
        log::info!("sample rate: {}", config.sample_rate().0);
        log::info!("num channels: {}", config.channels());
        let config = StreamConfig::from(config);
        Ok(Self { device, config })
    }

    /// Play an audio stream where samples are calculated with synchronous control flow with
    /// filling the audio buffer. This will have the lowest possible latency but possibly lower
    /// maximum throughput compared to other ways of playing a signal. It's also inflexible as it
    /// needs to own the signal being played.
    pub fn play_signal_sync<T, S>(&self, mut signal: S) -> anyhow::Result<()>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
        S: Signal<Item = T, SampleBuffer = Vec<T>>,
    {
        enum Command {
            RequestSamples(usize),
        }
        // channel for cpal thread to send messages to main thread
        let (send_command, recv_command) = mpsc::channel::<Command>();
        // buffer for sending samples from main thread to cpal thread
        let buf = Arc::new(RwLock::new(Vec::<T>::new()));
        let stream = self.device.build_output_stream(
            &self.config,
            {
                let buf = Arc::clone(&buf);
                let channels = self.config.channels;
                move |data: &mut [f32], _: &OutputCallbackInfo| {
                    send_command
                        .send(Command::RequestSamples(
                            data.len() / channels as usize,
                        ))
                        .expect("main thread stopped listening on channel");
                    let buf = buf.read().expect("main thread has stopped");
                    for (output, &input) in
                        data.chunks_mut(channels as usize).zip(buf.iter())
                    {
                        for element in output {
                            *element = input.to_f32();
                        }
                    }
                }
            },
            |err| eprintln!("stream error: {}", err),
            None,
        )?;
        stream.play()?;
        let mut ctx = SignalCtx {
            sample_rate_hz: self.config.sample_rate.0 as f64,
            batch_index: 0,
        };
        loop {
            match recv_command
                .recv()
                .expect("cpal thread stopped unexpectedly")
            {
                Command::RequestSamples(n) => {
                    {
                        // sample the signal directly into the buffer shared with the cpal thread
                        let mut buf = buf.write().unwrap();
                        buf.clear();
                        signal.sample_batch(&ctx, n, &mut *buf);
                    }
                    ctx.batch_index += 1;
                }
            }
        }
    }
}
