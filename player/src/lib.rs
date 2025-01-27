use caw_core::{SigCtx, SigSampleIntoBufT, Stereo};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Device, OutputCallbackInfo, Stream, StreamConfig,
    SupportedBufferSize,
};
use std::{
    collections::VecDeque,
    sync::{mpsc, Arc, Mutex, RwLock},
};

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
pub struct ConfigSync {
    /// default: 0.01
    pub system_latency_s: f32,
}

impl Default for ConfigSync {
    fn default() -> Self {
        Self {
            system_latency_s: 0.01,
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

    fn choose_config(
        &self,
        system_latency_s: f32,
    ) -> anyhow::Result<StreamConfig> {
        let default_config = self.device.default_output_config()?;
        let sample_rate = default_config.sample_rate();
        let channels = 2;
        let ideal_buffer_size =
            (sample_rate.0 as f32 * system_latency_s) as u32 * channels;
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
        config: ConfigSync,
    ) -> anyhow::Result<(Stream, StreamConfig)>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
    {
        let config = self.choose_config(config.system_latency_s)?;
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
        Ok((stream, config))
    }

    fn play_signal_sync_mono_callback_raw<T, S, F>(
        &self,
        mut sig: S,
        mut f: F,
        config: ConfigSync,
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
        let (stream, config) = self.make_stream_sync_mono(
            Arc::clone(&buffer),
            send_sync_command_request_num_samples,
            recv_sync_command_done,
            config,
        )?;
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
        config: ConfigSync,
    ) -> anyhow::Result<()>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
        S: SigSampleIntoBufT<Item = T>,
    {
        self.play_signal_sync_mono_callback_raw(signal, |_| (), config)
    }

    /// Like `play_signal_sync_mono` but calls a provided function on the data produced by the signal
    pub fn play_signal_sync_mono_callback<T, S, F>(
        &self,
        signal: S,
        mut f: F,
        config: ConfigSync,
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
        config: ConfigSync,
    ) -> anyhow::Result<(Stream, StreamConfig)>
    where
        TL: ToF32 + Send + Sync + Copy + 'static,
        TR: ToF32 + Send + Sync + Copy + 'static,
    {
        let config = self.choose_config(config.system_latency_s)?;
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
        Ok((stream, config))
    }

    fn play_signal_sync_stereo_callback_raw<TL, TR, SL, SR, F>(
        &self,
        mut sig: Stereo<SL, SR>,
        mut f: F,
        config: ConfigSync,
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
        let (stream, config) = self.make_stream_sync_stereo(
            buffer.map_ref(Arc::clone, Arc::clone),
            send_sync_command_request_num_samples,
            recv_sync_command_done,
            config,
        )?;
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
        config: ConfigSync,
    ) -> anyhow::Result<()>
    where
        TL: ToF32 + Send + Sync + Copy + 'static,
        TR: ToF32 + Send + Sync + Copy + 'static,
        SL: SigSampleIntoBufT<Item = TL>,
        SR: SigSampleIntoBufT<Item = TR>,
    {
        self.play_signal_sync_stereo_callback_raw(sig, |_| (), config)
    }

    /// Like `play_signal_sync_stereo` but calls a provided function on the data produced by the signal
    pub fn play_signal_sync_stereo_callback<TL, TR, SL, SR, F>(
        &self,
        sig: Stereo<SL, SR>,
        mut f: F,
        config: ConfigSync,
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

    pub fn into_async_stereo(
        self,
        config: ConfigAsync,
    ) -> anyhow::Result<PlayerAsyncStereo> {
        let queue = Stereo {
            left: Arc::new(Mutex::new(VecDeque::new())),
            right: Arc::new(Mutex::new(VecDeque::new())),
        };
        let stream_config = self.choose_config(config.system_latency_s)?;
        log::info!("sample rate: {}", stream_config.sample_rate.0);
        log::info!("num channels: {}", stream_config.channels);
        log::info!("buffer size: {:?}", stream_config.buffer_size);
        let stream = self.device.build_output_stream(
            &stream_config,
            {
                let queue = queue.map_ref(Arc::clone, Arc::clone);
                move |data: &mut [f32], _: &OutputCallbackInfo| {
                    let mut queue = queue.map_ref(
                        |l| l.lock().expect("main thread has stopped"),
                        |r| r.lock().expect("main thread has stopped"),
                    );
                    if data.len() / 2 > queue.left.len() {
                        log::warn!(
                            "Too few samples in queue. Needs {}, has {}.",
                            data.len() / 2,
                            queue.left.len()
                        );
                    }
                    for output_by_channel in
                        data.chunks_mut(stream_config.channels as usize)
                    {
                        output_by_channel[0] =
                            queue.left.pop_front().unwrap_or_default();
                        output_by_channel[1] =
                            queue.right.pop_front().unwrap_or_default()
                    }
                }
            },
            |err| eprintln!("stream error: {}", err),
            None,
        )?;
        stream.play()?;
        let max_num_samples = (config.queue_latency_s
            * stream_config.sample_rate.0 as f32)
            as usize;
        Ok(PlayerAsyncStereo {
            stream,
            queue,
            max_num_samples,
            scratch: Stereo::new(Vec::new(), Vec::new()),
            batch_index: 0,
            sample_rate_hz: stream_config.sample_rate.0 as f32,
        })
    }

    pub fn into_async_mono(
        self,
        config: ConfigAsync,
    ) -> anyhow::Result<PlayerAsyncMono> {
        let queue = Arc::new(Mutex::new(VecDeque::new()));
        let stream_config = self.choose_config(config.system_latency_s)?;
        log::info!("sample rate: {}", stream_config.sample_rate.0);
        log::info!("num channels: {}", stream_config.channels);
        log::info!("buffer size: {:?}", stream_config.buffer_size);
        let stream = self.device.build_output_stream(
            &stream_config,
            {
                let queue = Arc::clone(&queue);
                move |data: &mut [f32], _: &OutputCallbackInfo| {
                    let mut queue =
                        queue.lock().expect("main thread has stopped");
                    for output in
                        data.chunks_mut(stream_config.channels as usize)
                    {
                        let sample: f32 = queue.pop_front().unwrap_or_default();
                        for element in output {
                            *element = sample;
                        }
                    }
                }
            },
            |err| eprintln!("stream error: {}", err),
            None,
        )?;
        stream.play()?;
        let max_num_samples = (config.queue_latency_s
            * stream_config.sample_rate.0 as f32)
            as usize;
        Ok(PlayerAsyncMono {
            stream,
            queue,
            max_num_samples,
            scratch: Vec::new(),
            batch_index: 0,
            sample_rate_hz: stream_config.sample_rate.0 as f32,
        })
    }

    pub fn into_owned_stereo<SL, SR>(
        self,
        mut sig: Stereo<SL, SR>,
        config: ConfigSync,
    ) -> anyhow::Result<PlayerOwned>
    where
        SL: SigSampleIntoBufT<Item = f32> + Send + Sync + 'static,
        SR: SigSampleIntoBufT<Item = f32> + Send + Sync + 'static,
    {
        let stream_config = self.choose_config(config.system_latency_s)?;
        log::info!("sample rate: {}", stream_config.sample_rate.0);
        log::info!("num channels: {}", stream_config.channels);
        log::info!("buffer size: {:?}", stream_config.buffer_size);
        let mut ctx = SigCtx {
            sample_rate_hz: stream_config.sample_rate.0 as f32,
            batch_index: 0,
            num_samples: 0,
        };
        let channels = stream_config.channels;
        let data_copy = Arc::new(RwLock::new(Vec::new()));
        assert!(channels >= 2);
        let stream = {
            let data_copy = Arc::clone(&data_copy);
            self.device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &OutputCallbackInfo| {
                    ctx.batch_index += 1;
                    ctx.num_samples = data.len() / channels as usize;
                    sig.left.sample_into_slice(
                        &ctx,
                        channels as usize,
                        0,
                        data,
                    );
                    sig.right.sample_into_slice(
                        &ctx,
                        channels as usize,
                        1,
                        data,
                    );
                    {
                        // copy the data out so it can be visualized
                        let mut data_copy = data_copy.write().unwrap();
                        data_copy.resize(data.len(), 0.0);
                        data_copy.copy_from_slice(data);
                    }
                },
                |err| eprintln!("stream error: {}", err),
                None,
            )?
        };
        stream.play()?;
        Ok(PlayerOwned {
            stream,
            data: data_copy,
        })
    }

    pub fn into_owned_mono<S>(
        self,
        mut sig: S,
        config: ConfigSync,
    ) -> anyhow::Result<PlayerOwned>
    where
        S: SigSampleIntoBufT<Item = f32> + Send + Sync + 'static,
    {
        let stream_config = self.choose_config(config.system_latency_s)?;
        log::info!("sample rate: {}", stream_config.sample_rate.0);
        log::info!("num channels: {}", stream_config.channels);
        log::info!("buffer size: {:?}", stream_config.buffer_size);
        let mut ctx = SigCtx {
            sample_rate_hz: stream_config.sample_rate.0 as f32,
            batch_index: 0,
            num_samples: 0,
        };
        let channels = stream_config.channels;
        let data_copy = Arc::new(RwLock::new(Vec::new()));
        let stream = {
            let data_copy = Arc::clone(&data_copy);
            self.device.build_output_stream(
                &stream_config,
                move |data: &mut [f32], _: &OutputCallbackInfo| {
                    ctx.batch_index += 1;
                    ctx.num_samples = data.len() / channels as usize;
                    let mut data_copy = data_copy.write().unwrap();
                    sig.sample_into_buf(&ctx, &mut data_copy);
                    for (chunks, sample) in data
                        .chunks_exact_mut(channels as usize)
                        .zip(data_copy.iter())
                    {
                        for out in chunks {
                            *out = *sample;
                        }
                    }
                },
                |err| eprintln!("stream error: {}", err),
                None,
            )?
        };
        stream.play()?;
        Ok(PlayerOwned {
            stream,
            data: data_copy,
        })
    }
}

pub struct PlayerOwned {
    #[allow(unused)]
    stream: Stream,
    data: Arc<RwLock<Vec<f32>>>,
}

impl PlayerOwned {
    pub fn with_latest_data<F>(&self, mut f: F)
    where
        F: FnMut(&[f32]),
    {
        f(&self.data.read().unwrap());
    }
}

type StereoSharedAsyncQueue =
    Stereo<Arc<Mutex<VecDeque<f32>>>, Arc<Mutex<VecDeque<f32>>>>;

pub struct ConfigAsync {
    /// default: 1.0 / 30.0
    pub queue_latency_s: f32,
    /// default: 0.01
    pub system_latency_s: f32,
}

impl Default for ConfigAsync {
    fn default() -> Self {
        Self {
            queue_latency_s: 1.0 / 30.0,
            system_latency_s: 0.01,
        }
    }
}

pub struct PlayerAsyncStereo {
    #[allow(unused)]
    stream: Stream,
    queue: StereoSharedAsyncQueue,
    max_num_samples: usize,
    scratch: Stereo<Vec<f32>, Vec<f32>>,
    batch_index: u64,
    sample_rate_hz: f32,
}

impl PlayerAsyncStereo {
    pub fn play_signal_callback<SL, SR, F>(
        &mut self,
        sig: &mut Stereo<SL, SR>,
        mut f: F,
    ) where
        SL: SigSampleIntoBufT<Item = f32>,
        SR: SigSampleIntoBufT<Item = f32>,
        F: FnMut(Stereo<&[f32], &[f32]>),
    {
        // Assume that both queues are the same length.
        // Make sure that at least 1 sample is resolved each frame. Without this, interactive
        // triggers might be skipped if the buffer happens to be full when this method is called.
        // This means the latency is not necessarily fixed.
        let num_samples = self
            .max_num_samples
            .saturating_sub(self.queue.left.lock().unwrap().len())
            .max(1);
        let ctx = SigCtx {
            sample_rate_hz: self.sample_rate_hz,
            batch_index: self.batch_index,
            num_samples,
        };
        sig.sample_into_buf(&ctx, self.scratch.as_mut());
        f(self.scratch.map_ref(|v| v.as_slice(), |v| v.as_slice()));
        {
            let mut left = self.queue.left.lock().unwrap();
            for &x in &self.scratch.left {
                left.push_back(x);
            }
        }
        {
            let mut right = self.queue.right.lock().unwrap();
            for &x in &self.scratch.right {
                right.push_back(x);
            }
        }
        self.batch_index += 1;
    }

    pub fn play_signal<SL, SR>(&mut self, sig: &mut Stereo<SL, SR>)
    where
        SL: SigSampleIntoBufT<Item = f32>,
        SR: SigSampleIntoBufT<Item = f32>,
    {
        self.play_signal_callback(sig, |_| ());
    }
}

type MonoSharedAsyncQueue = Arc<Mutex<VecDeque<f32>>>;

pub struct PlayerAsyncMono {
    #[allow(unused)]
    stream: Stream,
    queue: MonoSharedAsyncQueue,
    max_num_samples: usize,
    scratch: Vec<f32>,
    batch_index: u64,
    sample_rate_hz: f32,
}

impl PlayerAsyncMono {
    pub fn play_signal_callback<S, F>(&mut self, sig: &mut S, mut f: F)
    where
        S: SigSampleIntoBufT<Item = f32>,
        F: FnMut(&[f32]),
    {
        // Make sure that at least 1 sample is resolved each frame. Without this, interactive
        // triggers might be skipped if the buffer happens to be full when this method is called.
        // This means the latency is not necessarily fixed.
        let num_samples = self
            .max_num_samples
            .saturating_sub(self.queue.lock().unwrap().len())
            .max(1);
        let ctx = SigCtx {
            sample_rate_hz: self.sample_rate_hz,
            batch_index: self.batch_index,
            num_samples,
        };
        sig.sample_into_buf(&ctx, self.scratch.as_mut());
        f(self.scratch.as_slice());
        let mut queue = self.queue.lock().unwrap();
        for &sample in &self.scratch {
            queue.push_back(sample);
        }
        self.batch_index += 1;
    }

    pub fn play_signal<S>(&mut self, sig: &mut S)
    where
        S: SigSampleIntoBufT<Item = f32>,
    {
        self.play_signal_callback(sig, |_| ());
    }
}
