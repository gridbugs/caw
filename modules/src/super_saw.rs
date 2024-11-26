use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Sig, SigCtx, SigT};
use itertools::izip;
use rand::{rngs::StdRng, Rng, SeedableRng};
use wide::f32x8;

#[derive(Debug, Clone, Copy)]
pub enum SuperSawInit {
    Random,
    Zero,
}

pub struct SuperSaw<F, D>
where
    F: SigT<Item = f32>,
    D: SigT<Item = f32>,
{
    freq_hz: F,
    detune_ratio: D,
    state_01: Vec<f32x8>,
    // Used to deal with the case where the number of oscillators is not divisible by 8. Each
    // element is either 0 or 1, with 0s in the positions of the left over values in the final
    // f32x8 in state_01.
    mask: Vec<f32x8>,
    // Also used to deal with the case where the number of oscillators is not divisible by 8.
    // Stores half of the number of oscillators in the corresponding f32x8
    offset_per_f32x8: Vec<f32>,
    num_oscillators: usize,
    buf: Vec<f32>,
}

impl<F, D> SigT for SuperSaw<F, D>
where
    F: SigT<Item = f32>,
    D: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        const RANGE_0_7: [f32; 8] = [0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0];
        let range_0_7: f32x8 = RANGE_0_7.into();
        // scales the sum of num_oscillators values in the range 0..1 to a value in the range -1..1
        let scale_factor = 2.0 / (self.num_oscillators as f32);
        self.buf.resize(ctx.num_samples, 0.0);
        for (sample, freq_hz, detune_ratio) in izip! {
            self.buf.iter_mut(),
            self.freq_hz.sample(ctx).iter(),
            self.detune_ratio.sample(ctx).iter(),
        } {
            *sample = 0.0;
            let (mut freq_hz, detune_step_hz) = if self.num_oscillators == 1 {
                (f32x8::splat(freq_hz), 0.0)
            } else {
                let detune_hz = freq_hz * detune_ratio;
                // Split the detune evenly among each oscillator.
                let detune_step_hz =
                    detune_hz / ((self.num_oscillators - 1) as f32);
                let detune_offsets_hz = range_0_7 * detune_step_hz;
                let min_freq_hz = freq_hz - (detune_hz / 2.0);
                (detune_offsets_hz + min_freq_hz, detune_step_hz)
            };
            for ((state_01, mask), offset) in self
                .state_01
                .iter_mut()
                .zip(self.mask.iter())
                .zip(self.offset_per_f32x8.iter())
            {
                let state_delta = freq_hz / ctx.sample_rate_hz;
                // Continue the ramp of the saw wave.
                *state_01 += state_delta * mask;
                // Subtract the rounded-down value from itself so that the wave wraps around from 1
                // to 0.
                *state_01 = *state_01 - (*state_01 - 0.5).round();
                let total_with_offset = state_01.reduce_add() - offset;
                *sample += total_with_offset * scale_factor;
                freq_hz = freq_hz + (detune_step_hz * 8.0);
            }
        }
        &self.buf
    }
}

impl<F, D> SuperSaw<F, D>
where
    F: SigT<Item = f32>,
    D: SigT<Item = f32>,
{
    pub fn new(
        freq_hz: F,
        detune_ratio: D,
        num_oscillators: usize,
        init: SuperSawInit,
    ) -> Sig<Self> {
        assert!(
            num_oscillators >= 1,
            "There must be at least one oscillator"
        );
        let num_f32x8s = num_oscillators / 8;
        let mut state_01 = vec![f32x8::ZERO; num_f32x8s];
        let mut mask = vec![f32x8::ONE; num_f32x8s];
        let mut offset_per_f32x8 = vec![4.0; num_f32x8s];
        if num_oscillators % 8 != 0 {
            state_01.push(f32x8::ZERO);
            let mut final_mask = [0.0; 8];
            for x in final_mask.iter_mut().take(num_oscillators % 8) {
                *x = 1.0;
            }
            mask.push(final_mask.into());
            offset_per_f32x8.push((num_oscillators % 8) as f32 / 2.0);
        }
        match init {
            SuperSawInit::Zero => (),
            SuperSawInit::Random => {
                let mut rng = StdRng::from_entropy();
                for (x, mask) in state_01.iter_mut().zip(mask.iter()) {
                    *x = rng.gen::<[f32; 8]>().into();
                    *x *= mask;
                }
            }
        }
        Sig(Self {
            freq_hz,
            detune_ratio,
            state_01,
            mask,
            offset_per_f32x8,
            num_oscillators,
            buf: Vec::new(),
        })
    }
}

builder! {
    #[constructor = "super_saw"]
    #[constructor_doc = "The mean of multiple saw wave oscillators with slight differences in frequency"]
    #[build_fn = "SuperSaw::new"]
    #[build_ty = "Sig<SuperSaw<F, D>>"]
    #[generic_setter_type_name = "X"]
    pub struct SuperSawBuilder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "F"]
        freq_hz: _,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[default = 0.01]
        #[generic_name = "D"]
        detune_ratio: f32,
        #[default = 16]
        num_oscillators: usize,
        #[default = SuperSawInit::Random]
        init: SuperSawInit,
    }
}
