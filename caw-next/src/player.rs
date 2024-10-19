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

struct SyncCommandRequestNumSamples(usize);
struct SyncCommandDone;

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

    fn make_stream_sync<T>(
        &self,
        buf: Arc<RwLock<Vec<T>>>,
        send_sync_command_request_num_samples: mpsc::Sender<
            SyncCommandRequestNumSamples,
        >,
        recv_sync_command_done: mpsc::Receiver<SyncCommandDone>,
    ) -> Result<cpal::Stream, cpal::BuildStreamError>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
    {
        self.device.build_output_stream(
            &self.config,
            {
                let channels = self.config.channels;
                move |data: &mut [f32], _: &OutputCallbackInfo| {
                    send_sync_command_request_num_samples
                        .send(SyncCommandRequestNumSamples(
                            data.len() / channels as usize,
                        ))
                        .expect("main thread stopped listening on channel");
                    let SyncCommandDone = recv_sync_command_done
                        .recv()
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
        )
    }

    fn play_signal_sync_callback_raw<T, S, F>(
        &self,
        mut signal: S,
        mut f: F,
    ) -> anyhow::Result<()>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
        S: Signal<Item = T, SampleBuffer = Vec<T>>,
        F: FnMut(&Arc<RwLock<Vec<T>>>),
    {
        // channel for cpal thread to send messages to main thread
        let (
            send_sync_command_request_num_samples,
            recv_sync_command_request_num_samples,
        ) = mpsc::channel::<SyncCommandRequestNumSamples>();
        let (send_sync_command_done, recv_sync_command_done) =
            mpsc::channel::<SyncCommandDone>();
        // buffer for sending samples from main thread to cpal thread
        let buf = Arc::new(RwLock::new(Vec::<T>::new()));
        let stream = self.make_stream_sync(
            Arc::clone(&buf),
            send_sync_command_request_num_samples,
            recv_sync_command_done,
        )?;
        stream.play()?;
        let mut ctx = SignalCtx {
            sample_rate_hz: self.config.sample_rate.0 as f64,
            batch_index: 0,
        };
        loop {
            let SyncCommandRequestNumSamples(num_samples) =
                recv_sync_command_request_num_samples
                    .recv()
                    .expect("cpal thread stopped unexpectedly");
            {
                // sample the signal directly into the buffer shared with the cpal thread
                let mut buf = buf.write().unwrap();
                send_sync_command_done
                    .send(SyncCommandDone)
                    .expect("cpal thread stopped unexpectedly");
                buf.clear();
                signal.sample_batch(&ctx, num_samples, &mut *buf);
            }
            f(&buf);
            ctx.batch_index += 1;
        }
    }

    /// Play an audio stream where samples are calculated with synchronous control flow while
    /// filling the audio buffer. This will have the lowest possible latency but possibly lower
    /// maximum throughput compared to other ways of playing a signal. It's also inflexible as it
    /// needs to own the signal being played.
    pub fn play_signal_sync<T, S>(&self, signal: S) -> anyhow::Result<()>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
        S: Signal<Item = T, SampleBuffer = Vec<T>>,
    {
        self.play_signal_sync_callback_raw(signal, |_| ())
    }

    /// Like `play_signal_sync` but calls a provided function on the data produced by the signal
    pub fn play_signal_sync_callback<T, S, F>(
        &self,
        signal: S,
        mut f: F,
    ) -> anyhow::Result<()>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
        S: Signal<Item = T, SampleBuffer = Vec<T>>,
        F: FnMut(&[T]),
    {
        self.play_signal_sync_callback_raw(signal, |buf| {
            let buf = buf.read().unwrap();
            f(&*buf);
        })
    }
}
