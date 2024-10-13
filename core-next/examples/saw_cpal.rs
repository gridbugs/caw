use caw_builders::*;
use caw_core_next::*;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    OutputCallbackInfo, StreamConfig,
};
use std::sync::mpsc;

fn signal() -> impl Signal<Item = f64> {
    oscillator(waveform::Saw, freq_hz(40.0))
        .build()
        .zip(oscillator(waveform::Saw, freq_hz(40.1)).build())
        .map(|(a, b)| (a + b) / 10.0)
}

enum Command {
    RequestSamples(usize),
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
    let mut buffered_signal = signal().buffered();
    let mut ctx = SignalCtx {
        sample_rate_hz: config.sample_rate().0 as f64,
        batch_index: 0,
    };
    let channels = config.channels();
    let config = StreamConfig::from(config);
    let (send_data, recv_data) = mpsc::channel::<f32>();
    let (send_command, recv_command) = mpsc::channel::<Command>();
    let stream = device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &OutputCallbackInfo| {
                send_command
                    .send(Command::RequestSamples(
                        data.len() / channels as usize,
                    ))
                    .unwrap();
                for output in data.chunks_mut(channels as usize) {
                    match recv_data.recv() {
                        Ok(input) => {
                            for element in output {
                                *element = input;
                            }
                        }
                        Err(e) => {
                            eprintln!("error receiving data: {}", e);
                            break;
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
        match recv_command.recv() {
            Ok(command) => match command {
                Command::RequestSamples(n) => {
                    buffered_signal.sample_batch(&ctx, n);
                    ctx.batch_index += 1;
                    for &sample in buffered_signal.samples() {
                        send_data.send(sample as f32).unwrap();
                    }
                }
            },
            Err(e) => panic!("{}", e),
        }
    }
}
