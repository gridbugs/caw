use caw_core::{SigCtx, SigSampleIntoBufT, Stereo};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Device, OutputCallbackInfo, StreamConfig, SupportedBufferSize,
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

#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// default: 0.01
    pub target_latency_s: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            target_latency_s: 0.01,
        }
    }
}

pub struct Player {
    device: Device,
}

struct SyncCommandRequestNumSamples(usize);
struct SyncCommandDone;

type StereoSharedBuf<L, R> = Stereo<Arc<RwLock<Vec<L>>>, Arc<RwLock<Vec<R>>>>;

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
        Ok(Self { device })
    }

    fn choose_config(&self, config: Config) -> anyhow::Result<StreamConfig> {
        let default_config = self.device.default_output_config()?;
        let sample_rate = default_config.sample_rate();
        let channels = 2;
        let ideal_buffer_size =
            (sample_rate.0 as f32 * config.target_latency_s) as u32 * channels;
        // Round down to a multiple of 4. It's not clear why this is necessary but alsa complains
        // if the buffer size is not evenly divisible by 4.
        let ideal_buffer_size = ideal_buffer_size & (!3);
        let buffer_size = match default_config.buffer_size() {
            SupportedBufferSize::Range { min, max } => {
                let frame_count = if ideal_buffer_size < *min {
                    *min
                } else if ideal_buffer_size > *max {
                    *max
                } else {
                    ideal_buffer_size
                };
                BufferSize::Fixed(frame_count)
            }
            SupportedBufferSize::Unknown => BufferSize::Default,
        };
        Ok(StreamConfig {
            channels: channels as u16,
            sample_rate,
            buffer_size,
        })
    }

    fn make_stream_sync_mono<T>(
        &self,
        buf: Arc<RwLock<Vec<T>>>,
        send_sync_command_request_num_samples: mpsc::Sender<
            SyncCommandRequestNumSamples,
        >,
        recv_sync_command_done: mpsc::Receiver<SyncCommandDone>,
        config: Config,
    ) -> anyhow::Result<cpal::Stream>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
    {
        let config = self.choose_config(config)?;
        log::info!("sample rate: {}", config.sample_rate.0);
        log::info!("num channels: {}", config.channels);
        log::info!("buffer size: {:?}", config.buffer_size);
        let stream = self.device.build_output_stream(
            &config,
            {
                let channels = config.channels;
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
        )?;
        Ok(stream)
    }

    fn play_signal_sync_mono_callback_raw<T, S, F>(
        &self,
        mut sig: S,
        mut f: F,
        config: Config,
    ) -> anyhow::Result<()>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
        S: SigSampleIntoBufT<Item = T>,
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
        let buffer = Arc::new(RwLock::new(Vec::new()));
        let stream = self.make_stream_sync_mono(
            Arc::clone(&buffer),
            send_sync_command_request_num_samples,
            recv_sync_command_done,
            config,
        )?;
        let config = self.choose_config(config)?;
        stream.play()?;
        let mut ctx = SigCtx {
            sample_rate_hz: config.sample_rate.0 as f32,
            batch_index: 0,
            num_samples: 0,
        };
        loop {
            let SyncCommandRequestNumSamples(num_samples) =
                recv_sync_command_request_num_samples
                    .recv()
                    .expect("cpal thread stopped unexpectedly");
            {
                ctx.num_samples = num_samples;
                // sample the signal directly into the buffer shared with the cpal thread
                let mut buffer = buffer.write().unwrap();
                send_sync_command_done
                    .send(SyncCommandDone)
                    .expect("cpal thread stopped unexpectedly");
                sig.sample_into_buf(&ctx, &mut *buffer);
            }
            f(&buffer);
            ctx.batch_index += 1;
        }
    }

    /// Play an audio stream where samples are calculated with synchronous control flow while
    /// filling the audio buffer. This will have the lowest possible latency but possibly lower
    /// maximum throughput compared to other ways of playing a signal. It's also inflexible as it
    /// needs to own the signal being played.
    pub fn play_signal_sync_mono<T, S>(
        &self,
        signal: S,
        config: Config,
    ) -> anyhow::Result<()>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
        S: SigSampleIntoBufT<Item = T>,
    {
        self.play_signal_sync_mono_callback_raw(signal, |_| (), config)
    }

    /// Like `play_signal_sync` but calls a provided function on the data produced by the signal
    pub fn play_signal_sync_mono_callback<T, S, F>(
        &self,
        signal: S,
        mut f: F,
        config: Config,
    ) -> anyhow::Result<()>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
        S: SigSampleIntoBufT<Item = T>,
        F: FnMut(&[T]),
    {
        self.play_signal_sync_mono_callback_raw(
            signal,
            |buf| {
                let buf = buf.read().unwrap();
                f(&buf);
            },
            config,
        )
    }

    fn make_stream_sync_stereo<TL, TR>(
        &self,
        buf: StereoSharedBuf<TL, TR>,
        send_sync_command_request_num_samples: mpsc::Sender<
            SyncCommandRequestNumSamples,
        >,
        recv_sync_command_done: mpsc::Receiver<SyncCommandDone>,
        config: Config,
    ) -> anyhow::Result<cpal::Stream>
    where
        TL: ToF32 + Send + Sync + Copy + 'static,
        TR: ToF32 + Send + Sync + Copy + 'static,
    {
        let config = self.choose_config(config)?;
        log::info!("sample rate: {}", config.sample_rate.0);
        log::info!("num channels: {}", config.channels);
        log::info!("buffer size: {:?}", config.buffer_size);
        let channels = config.channels;
        assert!(channels >= 2);
        let stream = self.device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &OutputCallbackInfo| {
                send_sync_command_request_num_samples
                    .send(SyncCommandRequestNumSamples(
                        data.len() / channels as usize,
                    )) // 2 channels
                    .expect("main thread stopped listening on channel");
                let SyncCommandDone = recv_sync_command_done
                    .recv()
                    .expect("main thread stopped listening on channel");
                let buf = buf.map_ref(
                    |l| l.read().expect("main thread has stopped"),
                    |r| r.read().expect("main thread has stopped"),
                );
                let mut buf_iter = buf.map_ref(|l| l.iter(), |r| r.iter());
                for output_by_channel in data.chunks_mut(channels as usize) {
                    output_by_channel[0] =
                        buf_iter.left.next().unwrap().to_f32();
                    output_by_channel[1] =
                        buf_iter.right.next().unwrap().to_f32();
                }
            },
            |err| eprintln!("stream error: {}", err),
            None,
        )?;
        Ok(stream)
    }

    fn play_signal_sync_stereo_callback_raw<TL, TR, SL, SR, F>(
        &self,
        mut sig: Stereo<SL, SR>,
        mut f: F,
        config: Config,
    ) -> anyhow::Result<()>
    where
        TL: ToF32 + Send + Sync + Copy + 'static,
        TR: ToF32 + Send + Sync + Copy + 'static,
        SL: SigSampleIntoBufT<Item = TL>,
        SR: SigSampleIntoBufT<Item = TR>,
        F: FnMut(&Stereo<Arc<RwLock<Vec<TL>>>, Arc<RwLock<Vec<TR>>>>),
    {
        // channel for cpal thread to send messages to main thread
        let (
            send_sync_command_request_num_samples,
            recv_sync_command_request_num_samples,
        ) = mpsc::channel::<SyncCommandRequestNumSamples>();
        let (send_sync_command_done, recv_sync_command_done) =
            mpsc::channel::<SyncCommandDone>();
        // buffer for sending samples from main thread to cpal thread
        let buffer = Stereo {
            left: Arc::new(RwLock::new(Vec::new())),
            right: Arc::new(RwLock::new(Vec::new())),
        };
        let stream = self.make_stream_sync_stereo(
            buffer.map_ref(Arc::clone, Arc::clone),
            send_sync_command_request_num_samples,
            recv_sync_command_done,
            config,
        )?;
        let config = self.choose_config(config)?;
        stream.play()?;
        let mut ctx = SigCtx {
            sample_rate_hz: config.sample_rate.0 as f32,
            batch_index: 0,
            num_samples: 0,
        };
        loop {
            let SyncCommandRequestNumSamples(num_samples) =
                recv_sync_command_request_num_samples
                    .recv()
                    .expect("cpal thread stopped unexpectedly");
            {
                ctx.num_samples = num_samples;
                // sample the signal directly into the buffer shared with the cpal thread
                let mut buffer = buffer
                    .map_ref(|l| l.write().unwrap(), |r| r.write().unwrap());
                send_sync_command_done
                    .send(SyncCommandDone)
                    .expect("cpal thread stopped unexpectedly");
                let stereo_buffer =
                    Stereo::new(&mut *buffer.left, &mut *buffer.right);
                sig.sample_into_buf(&ctx, stereo_buffer);
            }
            f(&buffer);
            ctx.batch_index += 1;
        }
    }

    /// Play an audio stream where samples are calculated with synchronous control flow while
    /// filling the audio buffer. This will have the lowest possible latency but possibly lower
    /// maximum throughput compared to other ways of playing a signal. It's also inflexible as it
    /// needs to own the signal being played.
    pub fn play_signal_sync_stereo<TL, TR, SL, SR>(
        &self,
        sig: Stereo<SL, SR>,
        config: Config,
    ) -> anyhow::Result<()>
    where
        TL: ToF32 + Send + Sync + Copy + 'static,
        TR: ToF32 + Send + Sync + Copy + 'static,
        SL: SigSampleIntoBufT<Item = TL>,
        SR: SigSampleIntoBufT<Item = TR>,
    {
        self.play_signal_sync_stereo_callback_raw(sig, |_| (), config)
    }

    /// Like `play_signal_sync` but calls a provided function on the data produced by the signal
    pub fn play_signal_sync_stereo_callback<TL, TR, SL, SR, F>(
        &self,
        sig: Stereo<SL, SR>,
        mut f: F,
        config: Config,
    ) -> anyhow::Result<()>
    where
        TL: ToF32 + Send + Sync + Copy + 'static,
        TR: ToF32 + Send + Sync + Copy + 'static,
        SL: SigSampleIntoBufT<Item = TL>,
        SR: SigSampleIntoBufT<Item = TR>,
        F: FnMut(Stereo<&[TL], &[TR]>),
    {
        self.play_signal_sync_stereo_callback_raw(
            sig,
            |buf| {
                let left: &[TL] = &buf.left.read().unwrap();
                let right: &[TR] = &buf.right.read().unwrap();
                let s = Stereo { left, right };
                f(s)
            },
            config,
        )
    }
}
