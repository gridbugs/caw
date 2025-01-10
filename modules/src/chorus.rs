use crate::{
    low_level::linearly_interpolating_ring_buffer::LinearlyInterpolatingRingBuffer,
    oscillator::{oscillator, Oscillator},
    Sine, Waveform,
};
use caw_builder_proc_macros::builder;
use caw_core::{
    sig_shared, Buf, Channel, Filter, Sig, SigCtx, SigShared, SigT,
};
use itertools::izip;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChorusLfoOffset {
    None,
    /// LFOs for voices on one channel will be at offsets interleaved with the voices from the
    /// other channel.
    Interleave(Channel),
}

builder! {
    #[constructor = "chorus"]
    #[constructor_doc = "Mix the signal with pitch-shifted copies of itself to produce interference patterns"]
    #[generic_setter_type_name = "X"]
    pub struct Props {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "R"]
        #[default = 1.0]
        lfo_rate_hz: f32,
        #[generic_with_constraint = "Waveform"]
        #[generic_name = "W"]
        #[default = Sine]
        lfo_waveform: Sine,
        #[default = ChorusLfoOffset::None]
        lfo_offset: ChorusLfoOffset,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "DL"]
        #[default = 0.02]
        delay_s: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "DP"]
        #[default = 0.001]
        depth_s: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "F"]
        #[default = 0.5]
        feedback_ratio: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "M"]
        #[default = 0.5]
        mix_01: f32,
        #[default = 3]
        num_voices: usize,
    }
}

type Osc<W, F> = Oscillator<W, Sig<SigShared<F>>, f32, f32, bool>;

pub struct Chorus<S, R, W, DL, DP, F, M>
where
    S: SigT<Item = f32>,
    R: SigT<Item = f32>,
    W: Waveform,
    DL: SigT<Item = f32>,
    DP: SigT<Item = f32>,
    F: SigT<Item = f32>,
    M: SigT<Item = f32>,
{
    delay_s: DL,
    depth_s: DP,
    feedback_ratio: F,
    mix_01: M,
    buf: Vec<f32>,
    ring: LinearlyInterpolatingRingBuffer,
    lfos: Vec<Osc<W, R>>,
    lfo_bufs: Vec<Vec<f32>>,
    sig: S,
}

impl<R, W, DL, DP, F, M> Filter for Props<R, W, DL, DP, F, M>
where
    R: SigT<Item = f32>,
    W: Waveform,
    DL: SigT<Item = f32>,
    DP: SigT<Item = f32>,
    F: SigT<Item = f32>,
    M: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S>
        = Chorus<S, R, W, DL, DP, F, M>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        let lfo_rate_hz = sig_shared(self.lfo_rate_hz);
        let total_offset_01 = match self.lfo_offset {
            ChorusLfoOffset::None => 0.,
            ChorusLfoOffset::Interleave(Channel::Left) => 0.,
            ChorusLfoOffset::Interleave(Channel::Right) => {
                // half of the step between the offset of two voices
                0.5 / self.num_voices as f32
            }
        };
        let lfos: Vec<Osc<W, R>> = (0..self.num_voices)
            .map(|i| {
                let offset_01 =
                    total_offset_01 + ((i as f32) / self.num_voices as f32);
                oscillator(self.lfo_waveform, lfo_rate_hz.clone())
                    .reset_offset_01(offset_01)
                    .build()
                    .0
            })
            .collect::<Vec<_>>();
        let lfo_bufs: Vec<Vec<f32>> =
            (0..self.num_voices).map(|_| Vec::new()).collect::<Vec<_>>();
        Chorus {
            delay_s: self.delay_s,
            depth_s: self.depth_s,
            feedback_ratio: self.feedback_ratio,
            mix_01: self.mix_01,
            buf: Vec::new(),
            ring: LinearlyInterpolatingRingBuffer::new(44100),
            lfos,
            lfo_bufs,
            sig,
        }
    }
}

impl<S, R, W, DL, DP, F, M> SigT for Chorus<S, R, W, DL, DP, F, M>
where
    S: SigT<Item = f32>,
    R: SigT<Item = f32>,
    W: Waveform,
    DL: SigT<Item = f32>,
    DP: SigT<Item = f32>,
    F: SigT<Item = f32>,
    M: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        // Pre-compute all the oscillators values for the current frame. The number of oscillators
        // is not statically known so it can't be zipped along with the other signals in the next
        // loop.
        for (lfo, lfo_buf) in self.lfos.iter_mut().zip(self.lfo_bufs.iter_mut())
        {
            lfo.sample(ctx).clone_to_vec(lfo_buf);
        }
        self.buf.resize_with(ctx.num_samples, Default::default);
        for (i, (out, sample, delay_s, depth_s, mix_01, feedback_ratio)) in
            izip! {
                self.buf.iter_mut(),
                self.sig.sample(ctx).iter(),
                self.delay_s.sample(ctx).iter(),
                self.depth_s.sample(ctx).iter(),
                self.mix_01.sample(ctx).iter(),
                self.feedback_ratio.sample(ctx).iter(),
            }
            .enumerate()
        {
            let mut total_output = 0.;
            for lfo_buf in &self.lfo_bufs {
                let lfo_sample = lfo_buf[i];
                // `delay_s` is delay to the peak of the lfo. `lfo_sample * depth_s` will be
                // between `depth_s` and `-depth_s`. If we added `depth_s` to the lfo then it would
                // oscillate between 0 and `2 * depth_s`. Hence we need to add `delay_s + depth_s`
                // to the lfo.
                let total_delay_s = (lfo_sample * depth_s) + depth_s + delay_s;
                let index = total_delay_s * ctx.sample_rate_hz;
                let output_for_current_lfo = self.ring.query_resizing(index);
                total_output += output_for_current_lfo;
            }
            let mean_output = total_output / self.lfos.len() as f32;
            self.ring
                .insert((sample * mix_01) + (mean_output * feedback_ratio));
            *out = mean_output + (sample * (1.0 - mix_01));
        }
        &self.buf
    }
}
