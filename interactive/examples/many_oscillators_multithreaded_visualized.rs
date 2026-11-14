use caw_core::*;
use caw_interactive::window::{Visualization, Window};
use caw_player::PlayerConfig;
use rgb_int::Rgb24;
use std::{
    sync::{
        Arc, RwLock,
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

type SimdVector = wide::f32x16;
const SIMD_VECTOR_WIDTH: usize = 16;
const SIMD_VECTOR_HALF_WIDTH: f32 = SIMD_VECTOR_WIDTH as f32 / 2.;

struct SawOscillatorWide {
    state_01: SimdVector,
    freq: SimdVector,
    buf: Vec<f32>,
}

impl SigT for SawOscillatorWide {
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        let state_delta = self.freq / ctx.sample_rate_hz;
        self.buf.resize(ctx.num_samples, 0.0);
        for sample_mut in self.buf.iter_mut() {
            self.state_01 = (self.state_01 + state_delta).fract();
            *sample_mut = self
                .state_01
                .reduce_add()
                .algebraic_sub(SIMD_VECTOR_HALF_WIDTH);
        }
        &self.buf
    }
}

fn osc_wide(freq: SimdVector) -> Sig<impl SigT<Item = f32>> {
    let state_01 = [0.; SIMD_VECTOR_WIDTH];
    Sig(SawOscillatorWide {
        freq,
        state_01: SimdVector::new(state_01),
        buf: Vec::new(),
    })
}

fn signal_wide(
    num_oscillators: usize,
    thread_index: usize,
    num_threads: usize,
) -> Sig<impl SigT<Item = f32>> {
    let total_oscillators = num_threads * num_oscillators;
    let base_freq_hz = 30.0;
    let freq_hz = match thread_index % 2 {
        0 => base_freq_hz,
        1 => (base_freq_hz * 5.) / 4.,
        _ => unreachable!(),
    };
    let detune = 0.05;
    let min_freq_hz = freq_hz / (1.0 + detune);
    let max_freq_hz = freq_hz * (1.0 + detune);
    let range_freq_hz = max_freq_hz - min_freq_hz;
    let freqs = (0..num_oscillators)
        .map(|i| {
            let position_01 = ((num_oscillators * thread_index) + i) as f32
                / ((num_threads * num_oscillators) as f32);
            min_freq_hz + (position_01 * range_freq_hz)
        })
        .collect::<Vec<_>>();
    let scale = 1.0 / total_oscillators as f32;
    freqs
        .chunks_exact(SIMD_VECTOR_WIDTH)
        .map(|chunk| {
            let mut array = [0.0; SIMD_VECTOR_WIDTH];
            array.copy_from_slice(chunk);
            osc_wide(SimdVector::new(array))
        })
        .sum::<Sig<_>>()
        * scale
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
    num_oscillators_per_thread: usize,
    recv_query: Receiver<Query>,
    send_done: Sender<Done>,
    buf: Arc<RwLock<Vec<f32>>>,
) {
    thread::spawn(move || {
        let mut sig =
            signal_wide(num_oscillators_per_thread, index, num_threads);
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
    fn new(
        num_threads: usize,
        num_oscillators_per_thread: usize,
        thread_index_base: usize,
    ) -> Self {
        let mut thread_info = Vec::new();
        for i in 0..num_threads {
            let (send_query, recv_query) = mpsc::channel();
            let (send_done, recv_done) = mpsc::channel();
            let buf = Arc::new(RwLock::new(Vec::new()));
            run_thread(
                thread_index_base + i,
                num_threads,
                num_oscillators_per_thread,
                recv_query,
                send_done,
                Arc::clone(&buf),
            );
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
        self.buf.fill(0.0);
        for thread_info in &self.thread_info {
            let Done = thread_info.recv_done.recv().unwrap();
            let buf = thread_info.buf.read().unwrap();
            for (out, sample) in self.buf.iter_mut().zip(buf.iter()) {
                *out = out.algebraic_add(*sample);
            }
        }
        &self.buf
    }
}

/// Run with 'RUSTFLAGS="-C target-cpu=native"' for better performance.
fn main() -> anyhow::Result<()> {
    env_logger::init();
    let num_threads = 25;
    let num_oscillators_per_thread = 40000;
    let num_oscillators_per_thread =
        num_oscillators_per_thread / SIMD_VECTOR_WIDTH * SIMD_VECTOR_WIDTH;
    println!("threads: {num_threads}");
    println!("wide osc per thread: {num_oscillators_per_thread}");
    println!(
        "total oscillators: {}",
        num_threads * num_oscillators_per_thread
    );
    let window = Window::builder()
        .visualization(Visualization::StereoOscillographics)
        .fade(true)
        .scale(4.0)
        .stable(true)
        .line_width(2)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .build();
    let stereo_signal = Stereo::new_fn_channel(|channel| {
        let sig = match channel {
            Channel::Left => Sig(MultithreadedSignal::new(
                num_threads / 2,
                num_oscillators_per_thread,
                0,
            )),
            Channel::Right => Sig(MultithreadedSignal::new(
                num_threads / 2,
                num_oscillators_per_thread,
                num_threads / 2,
            )),
        };
        sig.map_ctx(|x, ctx| {
            // Scale by the log of the time to remove loud initial transient
            let scale = (ctx.batch_index as f32)
                .algebraic_mul(0.5)
                .algebraic_add(1.)
                .log2();
            x.algebraic_mul(scale)
        })
    });
    window.play_stereo(
        stereo_signal,
        PlayerConfig {
            ..Default::default()
        },
    )
}
