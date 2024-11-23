use caw_core_next::*;
use caw_modules::*;
use caw_next::player::Player;
use rand::Rng;
use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, RwLock,
    },
    thread,
};

fn osc(freq: f32) -> Sig<impl SigT<Item = f32>> {
    oscillator(waveform::Saw, freq)
        .reset_offset_01(rand::thread_rng().gen::<f32>())
        .build()
}

fn signal(
    thread_index: usize,
    num_threads: usize,
) -> Sig<impl SigT<Item = f32>> {
    let n = 2000;
    let total_oscillators = num_threads * n;
    (0..n)
        .map(move |i| {
            let i = i + (n * thread_index);
            let offset_total = 10.0;
            let offset =
                ((i as f32 / total_oscillators as f32) - 0.5) * offset_total;
            osc(40.0 + offset)
        })
        .sum::<Sig<_>>()
        .map(move |x| (x / (total_oscillators as f32)) * 100.0)
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
        let mut sig = signal(index, num_threads);
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
                *out = *sample;
            }
        }
        &self.buf
    }
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let player = Player::new()?;
    player
        .play_signal_sync_mono(MultithreadedSignal::new(20), Default::default())
}
