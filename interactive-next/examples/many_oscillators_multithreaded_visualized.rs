use caw_core_next::*;
use caw_interactive_next::window::Window;
use rand::Rng;
use rgb_int::Rgb24;
use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, RwLock,
    },
    thread,
};
use wide::f32x8;

struct SawOscillatorWide {
    state_01: f32x8,
    freq: f32x8,
    buf: Vec<f32>,
}

impl SigT for SawOscillatorWide {
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let state_delta = self.freq / ctx.sample_rate_hz;
        self.buf.resize(ctx.num_samples, 0.0);
        for sample_mut in self.buf.iter_mut() {
            self.state_01 += state_delta;
            self.state_01 = self.state_01 - (self.state_01 - 0.5).round();
            *sample_mut = self.state_01.reduce_add() - 4.0;
        }
        &self.buf
    }
}

fn osc_wide(freq: f32x8) -> Sig<impl SigT<Item = f32>> {
    let num_steps = 3.0;
    let mut rng = rand::thread_rng();
    Sig(SawOscillatorWide {
        freq,
        state_01: f32x8::splat(
            (rng.gen::<f32>() * num_steps).floor() / num_steps,
        ) * 0.1,
        buf: Vec::new(),
    })
}

fn signal_wide(
    n: usize,
    thread_index: usize,
    num_threads: usize,
) -> Sig<impl SigT<Item = f32>> {
    let total_oscillators = num_threads * n;
    let base_freq_hz = 60.0;
    let freq_hz = match thread_index {
        0..=2 => base_freq_hz,
        3..=5 => base_freq_hz * 2.0,
        6..=8 => base_freq_hz * 3.0,
        9..=11 => base_freq_hz * 4.0,
        _ => base_freq_hz,
    };
    let detune = 0.02;
    let min_freq_hz = freq_hz / (1.0 + detune);
    let max_freq_hz = freq_hz * (1.0 + detune);
    let range_freq_hz = max_freq_hz - min_freq_hz;
    let freqs = (0..n)
        .map(|i| {
            let position_01 =
                ((n * thread_index) + i) as f32 / ((num_threads * n) as f32);
            min_freq_hz + (position_01 * range_freq_hz)
        })
        .collect::<Vec<_>>();
    freqs
        .chunks_exact(8)
        .map(|chunk| {
            let mut array = [0.0; 8];
            array.copy_from_slice(chunk);
            osc_wide(f32x8::new(array))
        })
        .sum::<Sig<_>>()
        .map(move |x| (x / (total_oscillators as f32)) * 50.0)
}

struct Query {
    ctx: SigCtx,
}

struct Done;

struct ThreadInfo {
    send_query: Sender<Query>,
    recv_done: Receiver<Done>,
    buf: Arc<RwLock<Vec<f32>>>,
}

fn run_thread(
    index: usize,
    num_threads: usize,
    recv_query: Receiver<Query>,
    send_done: Sender<Done>,
    buf: Arc<RwLock<Vec<f32>>>,
) {
    thread::spawn(move || {
        let mut sig = signal_wide(25000, index, num_threads);
        loop {
            let Query { ctx } = recv_query.recv().unwrap();
            {
                let in_buf = sig.sample(&ctx);
                let mut buf = buf.write().unwrap();
                buf.clear();
                buf.extend(in_buf.iter());
            }
            send_done.send(Done).unwrap();
        }
    });
}

struct MultithreadedSignal {
    thread_info: Vec<ThreadInfo>,
    buf: Vec<f32>,
}

impl MultithreadedSignal {
    fn new(num_threads: usize) -> Self {
        let mut thread_info = Vec::new();
        for i in 0..num_threads {
            let (send_query, recv_query) = mpsc::channel();
            let (send_done, recv_done) = mpsc::channel();
            let buf = Arc::new(RwLock::new(Vec::new()));
            run_thread(i, num_threads, recv_query, send_done, Arc::clone(&buf));
            thread_info.push(ThreadInfo {
                send_query,
                recv_done,
                buf,
            });
        }
        Self {
            thread_info,
            buf: Vec::new(),
        }
    }
}

impl SigT for MultithreadedSignal {
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        for thread_info in &self.thread_info {
            thread_info.send_query.send(Query { ctx: *ctx }).unwrap();
        }
        self.buf.resize(ctx.num_samples, 0.0);
        for thread_info in &self.thread_info {
            let Done = thread_info.recv_done.recv().unwrap();
            let buf = thread_info.buf.read().unwrap();
            for (out, sample) in self.buf.iter_mut().zip(buf.iter()) {
                *out += *sample;
            }
        }
        &self.buf
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder()
        .scale(2.0)
        .stable(true)
        .spread(2)
        .line_width(4)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    window.play_mono(MultithreadedSignal::new(12))
}
