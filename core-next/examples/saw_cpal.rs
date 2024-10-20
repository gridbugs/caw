use caw_builders::*;
use caw_core_next::*;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    OutputCallbackInfo, StreamConfig,
};
use std::sync::{mpsc, Arc, RwLock};

fn signal(
) -> BufferedSignal<impl SignalTrait<Item = f32, SampleBuffer = Vec<f32>>> {
    oscillator(waveform::Saw, freq_hz(40.0))
        .build()
        .zip(oscillator(waveform::Saw, freq_hz(40.1)).build())
        .map(|(a, b)| (a + b) / 10.0)
}

enum Request {
    RequestSamples(usize),
}

enum Reply {
    Done,
}

fn main() {
    let host = cpal::default_host();
    println!("cpal host: {}", host.id().name());
    let device = host.default_output_device().unwrap();
    if let Ok(name) = device.name() {
        println!("cpal device: {}", name);
    } else {
        println!("cpal device: (no name)");
    }
    let config = device.default_output_config().unwrap();
    println!("sample format: {}", config.sample_format());
    println!("sample rate: {}", config.sample_rate().0);
    println!("num channels: {}", config.channels());
    let mut ctx = SignalCtx {
        sample_rate_hz: config.sample_rate().0 as f32,
        batch_index: 0,
    };
    let channels = config.channels();
    let config = StreamConfig::from(config);
    let (send_request, recv_request) = mpsc::channel::<Request>();
    let (send_reply, recv_reply) = mpsc::channel::<Reply>();
    let BufferedSignal { mut signal, buffer } = signal();
    let buffer = Arc::new(RwLock::new(buffer));
    let stream = device
        .build_output_stream(
            &config,
            {
                let buffer = Arc::clone(&buffer);
                move |data: &mut [f32], _: &OutputCallbackInfo| {
                    send_request
                        .send(Request::RequestSamples(
                            data.len() / channels as usize,
                        ))
                        .unwrap();
                    let Reply::Done = recv_reply.recv().unwrap();
                    let buffer = buffer.read().unwrap();
                    for (output, &input) in
                        data.chunks_mut(channels as usize).zip(buffer.iter())
                    {
                        for element in output {
                            *element = input as f32;
                        }
                    }
                }
            },
            |err| eprintln!("stream error: {}", err),
            None,
        )
        .unwrap();
    stream.play().unwrap();
    loop {
        match recv_request.recv() {
            Ok(request) => match request {
                Request::RequestSamples(n) => {
                    {
                        // sample the signal directly into the buffer shared with the cpal thread
                        let mut buffer = buffer.write().unwrap();
                        buffer.clear();
                        signal.sample_batch(&ctx, n, &mut *buffer);
                    }
                    send_reply.send(Reply::Done).unwrap();
                    ctx.batch_index += 1;
                }
            },
            Err(e) => panic!("{}", e),
        }
    }
}
