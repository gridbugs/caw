use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, OutputCallbackInfo, Stream, StreamConfig,
};
use std::sync::{mpsc, Arc, RwLock};

struct SamplePlayerCore {
    device: Device,
    config: StreamConfig,
}

impl SamplePlayerCore {
    fn new() -> anyhow::Result<Self> {
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
}

pub struct SamplePlayer {
    core: SamplePlayerCore,
    #[allow(unused)]
    stream: Stream,
    sender: mpsc::Sender<f32>,
    sink_cursor: Arc<RwLock<u64>>,
    volume: Arc<RwLock<f32>>,
    /// The target amount to over-fill the buffer to prevent gaps in the sample stream presented to
    /// the audio device. Increasing this value will increase the latency between updating the
    /// stream and hearing the result, but will reduce the chance that the device will run out of
    /// samples, resulting in choppy sound. This value will depend on how quickly (in realtitme)
    /// the application can add samples to the buffer (by calling `play_sample` or `play_stream`),
    /// so it's influenced by your computer's speed and how much work is being done between
    /// updating the buffer. It defaults to 1/20 of the sample rate.
    pub buffer_padding: u64,
    source_cursor: u64,
    downsample: u32,
}

impl SamplePlayer {
    pub fn new_with_downsample(downsample: u32) -> anyhow::Result<Self> {
        assert!(downsample > 0, "downsample must be positive");
        let (sender, receiver) = mpsc::channel::<f32>();
        let sink_cursor = Arc::new(RwLock::new(0));
        let sink_cursor_for_cpal_thread = Arc::clone(&sink_cursor);
        let volume = Arc::new(RwLock::new(1.0));
        let volume_for_cpal_thread = Arc::clone(&volume);
        let core = SamplePlayerCore::new()?;
        let channels = core.config.channels;
        let stream = core.device.build_output_stream(
            &core.config,
            move |data: &mut [f32], _: &OutputCallbackInfo| {
                let mut sink_cursor = sink_cursor_for_cpal_thread.write().unwrap();
                let volume = *volume_for_cpal_thread.read().unwrap();
                for output in data.chunks_mut(channels as usize * downsample as usize) {
                    if let Ok(input) = receiver.try_recv() {
                        for element in output {
                            *element = input * volume;
                        }
                        *sink_cursor += 1;
                    } else {
                        break;
                    }
                }
            },
            |err| log::error!("stream error: {}", err),
            None,
        )?;
        stream.play()?;
        let buffer_padding = core.config.sample_rate.0 as u64 / 20;
        Ok(Self {
            core,
            buffer_padding,
            stream,
            sender,
            sink_cursor,
            volume,
            source_cursor: 0,
            downsample,
        })
    }

    pub fn new() -> anyhow::Result<Self> {
        Self::new_with_downsample(1)
    }

    pub fn sample_rate_hz(&self) -> u32 {
        self.core.config.sample_rate.0 / self.downsample
    }

    fn play_sample(&mut self, sample: f32) {
        if let Err(_) = self.sender.send(sample) {
            log::error!("failed to send data to cpal thread");
        }
        self.source_cursor += 1;
    }

    fn samples_behind(&self) -> u64 {
        let sink_cursor = *self.sink_cursor.read().unwrap();
        let target_source_cursor = sink_cursor + self.buffer_padding;
        target_source_cursor - self.source_cursor
    }

    pub fn play_stream<S: FnMut() -> f32>(&mut self, mut stream: S) {
        // only send data once per channel
        for _ in 0..(self.samples_behind()) {
            self.play_sample(stream())
        }
    }

    pub fn set_volume(&self, volume: f32) {
        *self.volume.write().unwrap() = volume;
    }
}
