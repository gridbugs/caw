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
}

impl Sig for SawOscillatorWide {
    type Item = f32;
    type Buf = Vec<f32>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        let state_delta = self.freq / ctx.sample_rate_hz;
        sample_buffer.resize(ctx.num_samples, 0.0);
        for sample_mut in sample_buffer {
            self.state_01 += state_delta;
            self.state_01 = self.state_01 - (self.state_01 - 0.5).round();
            *sample_mut = self.state_01.reduce_add() - 4.0;
        }
    }
}

fn osc_wide(freq: f32x8) -> SigBuf<impl Sig<Item = f32>> {
    SawOscillatorWide {
        freq,
        state_01: if rand::thread_rng().gen::<f32>() < 0.5 {
            f32x8::splat(0.0)
        } else {
            f32x8::splat(0.1)
        },
    }
    .buffered()
}

fn signal_wide(
    n: usize,
    thread_index: usize,
    num_threads: usize,
) -> SigBuf<impl Sig<Item = f32, Buf = Vec<f32>>> {
    let total_oscillators = num_threads * n;
    let freqs = (0..n)
        .map(|i| {
            let i = i + (n * thread_index);
            let offset_total = 2.0;
            let offset =
                ((i as f32 / total_oscillators as f32) - 0.5) * offset_total;
            40.0 + offset
        })
        .collect::<Vec<_>>();
    freqs
        .chunks_exact(8)
        .map(|chunk| {
            let mut array = [0.0; 8];
            array.copy_from_slice(chunk);
            osc_wide(f32x8::new(array))
        })
        .sum::<SigBuf<_>>()
        .map(move |x| (x / (total_oscillators as f32)) * 200.0)
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
        let mut sig_buf = signal_wide(25000, index, num_threads);
        loop {
            let Query { ctx } = recv_query.recv().unwrap();
            {
                let mut buf = buf.write().unwrap();
                buf.clear();
                sig_buf.signal.sample_batch(&ctx, &mut *buf);
            }
            send_done.send(Done).unwrap();
        }
    });
}

struct MultithreadedSignal {
    thread_info: Vec<ThreadInfo>,
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
        Self { thread_info }
    }
}

impl Sig for MultithreadedSignal {
    type Item = f32;
    type Buf = Vec<f32>;

    fn sample_batch(&mut self, ctx: &SigCtx, sample_buffer: &mut Self::Buf) {
        for thread_info in &self.thread_info {
            thread_info.send_query.send(Query { ctx: *ctx }).unwrap();
        }
        sample_buffer.resize(ctx.num_samples, 0.0);
        for thread_info in &self.thread_info {
            let Done = thread_info.recv_done.recv().unwrap();
            let buf = thread_info.buf.read().unwrap();
            for (out, sample) in sample_buffer.iter_mut().zip(buf.iter()) {
                *out = *sample;
            }
        }
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
    window.play(MultithreadedSignal::new(8).buffered())
}
