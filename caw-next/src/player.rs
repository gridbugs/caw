use caw_core_next::{Sig, SigBuf, SigCtx};
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

    fn make_stream_sync_mono<T>(
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

    fn play_signal_sync_mono_callback_raw<T, S, F>(
        &self,
        buffered_signal: SigBuf<S>,
        mut f: F,
    ) -> anyhow::Result<()>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
        S: Sig<Item = T, Buf = Vec<T>>,
        F: FnMut(&Arc<RwLock<Vec<T>>>),
    {
        let SigBuf { mut signal, buffer } = buffered_signal;
        // channel for cpal thread to send messages to main thread
        let (
            send_sync_command_request_num_samples,
            recv_sync_command_request_num_samples,
        ) = mpsc::channel::<SyncCommandRequestNumSamples>();
        let (send_sync_command_done, recv_sync_command_done) =
            mpsc::channel::<SyncCommandDone>();
        // buffer for sending samples from main thread to cpal thread
        let buffer = Arc::new(RwLock::new(buffer));
        let stream = self.make_stream_sync_mono(
            Arc::clone(&buffer),
            send_sync_command_request_num_samples,
            recv_sync_command_done,
        )?;
        stream.play()?;
        let mut ctx = SigCtx {
            sample_rate_hz: self.config.sample_rate.0 as f32,
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
                buffer.clear();
                signal.sample_batch(&ctx, &mut *buffer);
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
        signal: SigBuf<S>,
    ) -> anyhow::Result<()>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
        S: Sig<Item = T, Buf = Vec<T>>,
    {
        self.play_signal_sync_mono_callback_raw(signal, |_| ())
    }

    /// Like `play_signal_sync` but calls a provided function on the data produced by the signal
    pub fn play_signal_sync_mono_callback<T, S, F>(
        &self,
        signal: SigBuf<S>,
        mut f: F,
    ) -> anyhow::Result<()>
    where
        T: ToF32 + Send + Sync + Copy + 'static,
        S: Sig<Item = T, Buf = Vec<T>>,
        F: FnMut(&[T]),
    {
        self.play_signal_sync_mono_callback_raw(signal, |buf| {
            let buf = buf.read().unwrap();
            f(&*buf);
        })
    }

    fn make_stream_sync_stereo<TL, TR>(
        &self,
        buf_left: Arc<RwLock<Vec<TL>>>,
        buf_right: Arc<RwLock<Vec<TR>>>,
        send_sync_command_request_num_samples: mpsc::Sender<
            SyncCommandRequestNumSamples,
        >,
        recv_sync_command_done: mpsc::Receiver<SyncCommandDone>,
    ) -> Result<cpal::Stream, cpal::BuildStreamError>
    where
        TL: ToF32 + Send + Sync + Copy + 'static,
        TR: ToF32 + Send + Sync + Copy + 'static,
    {
        let channels = self.config.channels;
        assert!(channels >= 2);
        self.device.build_output_stream(
            &self.config,
            move |data: &mut [f32], _: &OutputCallbackInfo| {
                send_sync_command_request_num_samples
                    .send(SyncCommandRequestNumSamples(
                        data.len() / channels as usize,
                    )) // 2 channels
                    .expect("main thread stopped listening on channel");
                let SyncCommandDone = recv_sync_command_done
                    .recv()
                    .expect("main thread stopped listening on channel");
                let buf_left =
                    buf_left.read().expect("main thread has stopped");
                let buf_right =
                    buf_right.read().expect("main thread has stopped");
                let mut buf_left_iter = buf_left.iter();
                let mut buf_right_iter = buf_right.iter();
                for output_by_channel in data.chunks_mut(channels as usize) {
                    output_by_channel[0] =
                        buf_left_iter.next().unwrap().to_f32();
                    output_by_channel[1] =
                        buf_right_iter.next().unwrap().to_f32();
                }
            },
            |err| eprintln!("stream error: {}", err),
            None,
        )
    }

    fn play_signal_sync_stereo_callback_raw<TL, TR, SL, SR, F>(
        &self,
        buffered_signal_left: SigBuf<SL>,
        buffered_signal_right: SigBuf<SR>,
        mut f: F,
    ) -> anyhow::Result<()>
    where
        TL: ToF32 + Send + Sync + Copy + 'static,
        TR: ToF32 + Send + Sync + Copy + 'static,
        SL: Sig<Item = TL, Buf = Vec<TL>>,
        SR: Sig<Item = TR, Buf = Vec<TR>>,
        F: FnMut(&Arc<RwLock<Vec<TL>>>, &Arc<RwLock<Vec<TR>>>),
    {
        let SigBuf {
            signal: mut signal_left,
            buffer: buffer_left,
        } = buffered_signal_left;
        let SigBuf {
            signal: mut signal_right,
            buffer: buffer_right,
        } = buffered_signal_right;
        // channel for cpal thread to send messages to main thread
        let (
            send_sync_command_request_num_samples,
            recv_sync_command_request_num_samples,
        ) = mpsc::channel::<SyncCommandRequestNumSamples>();
        let (send_sync_command_done, recv_sync_command_done) =
            mpsc::channel::<SyncCommandDone>();
        // buffer for sending samples from main thread to cpal thread
        let buffer_left = Arc::new(RwLock::new(buffer_left));
        let buffer_right = Arc::new(RwLock::new(buffer_right));
        let stream = self.make_stream_sync_stereo(
            Arc::clone(&buffer_left),
            Arc::clone(&buffer_right),
            send_sync_command_request_num_samples,
            recv_sync_command_done,
        )?;
        stream.play()?;
        let mut ctx = SigCtx {
            sample_rate_hz: self.config.sample_rate.0 as f32,
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
                let mut buffer_left = buffer_left.write().unwrap();
                let mut buffer_right = buffer_right.write().unwrap();
                send_sync_command_done
                    .send(SyncCommandDone)
                    .expect("cpal thread stopped unexpectedly");
                buffer_left.clear();
                signal_left.sample_batch(&ctx, &mut *buffer_left);
                buffer_right.clear();
                signal_right.sample_batch(&ctx, &mut *buffer_right);
            }
            f(&buffer_left, &buffer_right);
            ctx.batch_index += 1;
        }
    }

    /// Play an audio stream where samples are calculated with synchronous control flow while
    /// filling the audio buffer. This will have the lowest possible latency but possibly lower
    /// maximum throughput compared to other ways of playing a signal. It's also inflexible as it
    /// needs to own the signal being played.
    pub fn play_signal_sync_stereo<TL, TR, SL, SR>(
        &self,
        signal_left: SigBuf<SL>,
        signal_right: SigBuf<SR>,
    ) -> anyhow::Result<()>
    where
        TL: ToF32 + Send + Sync + Copy + 'static,
        TR: ToF32 + Send + Sync + Copy + 'static,
        SL: Sig<Item = TL, Buf = Vec<TL>>,
        SR: Sig<Item = TR, Buf = Vec<TR>>,
    {
        self.play_signal_sync_stereo_callback_raw(
            signal_left,
            signal_right,
            |_, _| (),
        )
    }

    /// Like `play_signal_sync` but calls a provided function on the data produced by the signal
    pub fn play_signal_sync_stereo_callback<TL, TR, SL, SR, F>(
        &self,
        signal_left: SigBuf<SL>,
        signal_right: SigBuf<SR>,
        mut f: F,
    ) -> anyhow::Result<()>
    where
        TL: ToF32 + Send + Sync + Copy + 'static,
        TR: ToF32 + Send + Sync + Copy + 'static,
        SL: Sig<Item = TL, Buf = Vec<TL>>,
        SR: Sig<Item = TR, Buf = Vec<TR>>,
        F: FnMut(&[TL], &[TR]),
    {
        self.play_signal_sync_stereo_callback_raw(
            signal_left,
            signal_right,
            |buf_left, buf_right| {
                let buf_left = buf_left.read().unwrap();
                let buf_right = buf_right.read().unwrap();
                f(&*buf_left, &*buf_right);
            },
        )
    }
}
