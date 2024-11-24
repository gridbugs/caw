// This implementation is based on the freeverb algorithm.
// Source code is here: https://github.com/sinshu/freeverb
// A description of the algorithm is here: https://ccrma.stanford.edu/~jos/pasp/Freeverb.html

struct Comb {
    feedback: f32,
    damp1: f32,
    damp2: f32,
    buffer: Vec<f32>,
    bufidx: usize,
    filter_store: f32,
}

struct CombArgs {
    feedback: f32,
    damping: f32,
    buffer_size: usize,
}

impl Comb {
    fn new(
        CombArgs {
            feedback,
            damping,
            buffer_size,
        }: CombArgs,
    ) -> Self {
        assert!(buffer_size > 0);
        Self {
            feedback,
            damp1: damping,
            damp2: 1.0 - damping,
            buffer: vec![0.0; buffer_size],
            bufidx: 0,
            filter_store: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.bufidx];
        self.filter_store =
            (output * self.damp2) + (self.filter_store * self.damp1);
        self.buffer[self.bufidx] = input + (self.filter_store * self.feedback);
        self.bufidx += 1;
        if self.bufidx == self.buffer.len() {
            self.bufidx = 0;
        }
        output
    }

    fn set_damping(&mut self, damping: f32) {
        self.damp1 = damping;
        self.damp2 = 1.0 - damping;
    }
}

struct AllPass {
    feedback: f32,
    buffer: Vec<f32>,
    bufidx: usize,
}

struct AllPassArgs {
    feedback: f32,
    buffer_size: usize,
}

impl AllPass {
    fn new(
        AllPassArgs {
            feedback,
            buffer_size,
        }: AllPassArgs,
    ) -> Self {
        assert!(buffer_size > 0);
        Self {
            feedback,
            buffer: vec![0.0; buffer_size],
            bufidx: 0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let bufout = self.buffer[self.bufidx];
        let output = bufout - input;
        self.buffer[self.bufidx] = input + (bufout * self.feedback);
        self.bufidx += 1;
        if self.bufidx == self.buffer.len() {
            self.bufidx = 0;
        }
        output
    }
}

mod tuning {
    pub const GAIN_SCALE: f32 = 0.015;
    pub const DAMPING_SCALE: f32 = 0.4;
    pub const INITIAL_DAMPING: f32 = 0.5;
    pub const SCALE_ROOM: f32 = 0.28;
    pub const OFFSET_ROOM: f32 = 0.7;
    pub const INITIAL_ROOM_SIZE: f32 = 0.5;
    pub const COMB_BUFFER_SIZES: &[usize] =
        &[1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
    pub const ALL_PASS_BUFFER_SIZES: &[usize] = &[556, 441, 341, 225];
    pub const ALL_PASS_FEEDBACK: f32 = 0.5;
}

pub use tuning::{INITIAL_DAMPING, INITIAL_ROOM_SIZE};

pub struct ReverbModel {
    comb: Vec<Comb>,
    all_pass: Vec<AllPass>,
}

fn room_size_to_comb_feedback(room_size: f32) -> f32 {
    (room_size * tuning::SCALE_ROOM) + tuning::OFFSET_ROOM
}

impl ReverbModel {
    pub fn new() -> Self {
        let comb_feedback =
            room_size_to_comb_feedback(tuning::INITIAL_ROOM_SIZE);
        let comb = tuning::COMB_BUFFER_SIZES
            .iter()
            .map(|&buffer_size| {
                Comb::new(CombArgs {
                    feedback: comb_feedback,
                    damping: tuning::INITIAL_DAMPING * tuning::DAMPING_SCALE,
                    buffer_size,
                })
            })
            .collect::<Vec<_>>();
        let all_pass = tuning::ALL_PASS_BUFFER_SIZES
            .iter()
            .map(|&buffer_size| {
                AllPass::new(AllPassArgs {
                    feedback: tuning::ALL_PASS_FEEDBACK,
                    buffer_size,
                })
            })
            .collect::<Vec<_>>();
        Self { comb, all_pass }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        let input = input * tuning::GAIN_SCALE;
        let mut out = 0.0;
        for comb in self.comb.iter_mut() {
            out += comb.process(input);
        }
        for all_pass in self.all_pass.iter_mut() {
            out = all_pass.process(out);
        }
        out
    }

    pub fn set_room_size(&mut self, room_size: f32) {
        let comb_feedback = room_size_to_comb_feedback(room_size);
        for comb in self.comb.iter_mut() {
            comb.feedback = comb_feedback;
        }
    }

    pub fn set_damping(&mut self, damping: f32) {
        for comb in self.comb.iter_mut() {
            comb.set_damping(damping * tuning::DAMPING_SCALE);
        }
    }
}
