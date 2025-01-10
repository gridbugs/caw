use caw_builder_proc_macros::builder;
use caw_core::{Buf, Sig, SigCtx, SigT};

pub struct SamplePlayback<P>
where
    P: SigT<Item = bool>,
{
    // controls whether the sample is currently playing
    play_gate: P,
    sample_buffer: Vec<f32>,
    index: usize,
    buf: Vec<f32>,
}

impl<P> SamplePlayback<P>
where
    P: SigT<Item = bool>,
{
    fn new(play_gate: P, sample_buffer: Vec<f32>) -> Sig<Self> {
        Sig(Self {
            play_gate,
            sample_buffer,
            index: 0,
            buf: Vec::new(),
        })
    }
}

impl<P> SigT for SamplePlayback<P>
where
    P: SigT<Item = bool>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        // Only consider the first value from the "play" signal each frame. This is to simplify
        // copying samples to the output buffer, but is technically incorrect in some rare edge
        // cases. This can be revisited later if it causes problems.
        if self.play_gate.sample(ctx).iter().next().unwrap() {
            let next_index =
                (self.index + ctx.num_samples).min(self.sample_buffer.len());
            let samples_this_frame =
                &self.sample_buffer[self.index..next_index];
            self.index = next_index;
            self.buf.resize(samples_this_frame.len(), 0.0);
            self.buf.copy_from_slice(samples_this_frame);
        } else {
            // This silences the module when the play signal is false.
            self.buf.clear();
        }
        // Pad the buffer with 0s so on the last frame the buffer is still the requested size.
        self.buf.resize(ctx.num_samples, 0.0);
        &self.buf
    }
}

builder! {
    #[constructor = "sample_playback"]
    #[constructor_doc = "Play back samples from a buffer"]
    #[build_fn = "SamplePlayback::new"]
    #[build_ty = "Sig<SamplePlayback<P>>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilder {
        #[generic_with_constraint = "SigT<Item = bool>"]
        #[generic_name = "P"]
        #[default = true]
        play_gate: bool,
        sample_buffer: Vec<f32>,
    }
}
