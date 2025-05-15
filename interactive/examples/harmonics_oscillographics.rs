use caw_core::*;
use caw_interactive::{
    Input,
    window::{Visualization, Window},
};
use caw_modules::*;
use rand::Rng;
use rgb_int::Rgb24;
use std::{
    sync::{
        Arc, RwLock,
        mpsc::{self, Receiver, Sender},
    },
    thread,
};

fn signal() -> Sig<impl SigT<Item = f32>> {
    let base_freq_hz = 50.0;
    let mut rng = rand::rng();
    let freq_hz = (rng.random_range(1..=3) as f32 * base_freq_hz)
        + (rng.random::<f32>() * 0.1);
    let lfo = oscillator(Saw, rng.random::<f32>() * 0.1).build()
        * rng.random::<f32>();
    oscillator(Saw, freq_hz + lfo).build() * 0.2
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
    recv_query: Receiver<Query>,
    send_done: Sender<Done>,
    buf: Arc<RwLock<Vec<f32>>>,
) {
    thread::spawn(move || {
        let mut sig = signal();
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
    fn new_sig(num_threads: usize, input: Input) -> Sig<impl SigT<Item = f32>> {
        let mut thread_info = Vec::new();
        for _ in 0..num_threads {
            let (send_query, recv_query) = mpsc::channel();
            let (send_done, recv_done) = mpsc::channel();
            let buf = Arc::new(RwLock::new(Vec::new()));
            run_thread(recv_query, send_done, Arc::clone(&buf));
            thread_info.push(ThreadInfo {
                send_query,
                recv_done,
                buf,
            });
        }
        let mix_01 = input.mouse.x_01().shared();
        Sig(Self {
            thread_info,
            buf: Vec::new(),
        })
        .filter(low_pass::default(2500.).resonance(1.0))
        .filter(
            compressor()
                .threshold(0.5)
                .scale(2.0)
                .ratio(input.mouse.y_01()),
        )
        .filter(
            chorus()
                .num_voices(1)
                .delay_s(0.001)
                .lfo_rate_hz(0.1)
                .feedback_ratio(0.5)
                .mix_01(mix_01.clone()),
        )
        .filter(reverb::default().room_size(0.9).damping(0.1).mix_01(mix_01))
        .filter(high_pass::default(1.0))
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
                *out += *sample;
            }
        }
        &self.buf
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let window = Window::builder()
        .scale(1.0)
        .line_width(2)
        .background(Rgb24::new(0, 31, 0))
        .foreground(Rgb24::new(0, 255, 0))
        .visualization(Visualization::StereoOscillographics)
        .build();
    let thresh = 10.0;
    let input = window.input();
    let sig = Stereo::new_fn(|| {
        MultithreadedSignal::new_sig(6, input.clone())
            .map(|x| x.clamp(-thresh, thresh))
    });
    window.play_stereo(sig, Default::default())
}
