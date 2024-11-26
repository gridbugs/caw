use crate::low_level::freeverb;
use caw_builder_proc_macros::builder;
use caw_core_next::{Buf, Filter, SigCtx, SigT};
use itertools::izip;

pub struct Props<R, D, M>
where
    R: SigT<Item = f32>,
    D: SigT<Item = f32>,
    M: SigT<Item = f32>,
{
    // controls reverb delay
    room_size: R,
    // removes high end of reverb to simulate a "softer room"
    damping: D,
    // 0 is dry signal, 1 is all reverb
    mix: M,
}

impl<R, D, M> Props<R, D, M>
where
    R: SigT<Item = f32>,
    D: SigT<Item = f32>,
    M: SigT<Item = f32>,
{
    fn new(room_size: R, damping: D, mix: M) -> Self {
        Self {
            room_size,
            damping,
            mix,
        }
    }
}

builder! {
    #[constructor = "reverb_freeverb"]
    #[constructor_doc = "Reverb module with adjustable room size and damping"]
    #[build_fn = "Props::new"]
    #[build_ty = "Props<R, D, M>"]
    #[generic_setter_type_name = "X"]
    pub struct PropsBuilder {
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "R"]
        #[default = freeverb::INITIAL_ROOM_SIZE as f32]
        room_size: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "D"]
        #[default = freeverb::INITIAL_DAMPING as f32]
        damping: f32,
        #[generic_with_constraint = "SigT<Item = f32>"]
        #[generic_name = "M"]
        #[default = 0.5]
        mix: f32,
    }
}

pub struct ReverbFreeverb<S, R, D, M>
where
    S: SigT<Item = f32>,
    R: SigT<Item = f32>,
    D: SigT<Item = f32>,
    M: SigT<Item = f32>,
{
    props: Props<R, D, M>,
    sig: S,
    state: freeverb::ReverbModel,
    room_size_prev: f32,
    damping_prev: f32,
    buf: Vec<f32>,
}

impl<R, D, M> Filter for PropsBuilder<R, D, M>
where
    R: SigT<Item = f32>,
    D: SigT<Item = f32>,
    M: SigT<Item = f32>,
{
    type ItemIn = f32;

    type Out<S> = ReverbFreeverb<S, R, D, M>
    where
        S: SigT<Item = Self::ItemIn>;

    fn into_sig<S>(self, sig: S) -> Self::Out<S>
    where
        S: SigT<Item = Self::ItemIn>,
    {
        let props = self.build();
        ReverbFreeverb {
            props,
            sig,
            state: freeverb::ReverbModel::new(),
            room_size_prev: freeverb::INITIAL_ROOM_SIZE as f32,
            damping_prev: freeverb::INITIAL_DAMPING as f32,
            buf: Vec::new(),
        }
    }
}

impl<S, R, D, M> SigT for ReverbFreeverb<S, R, D, M>
where
    S: SigT<Item = f32>,
    R: SigT<Item = f32>,
    D: SigT<Item = f32>,
    M: SigT<Item = f32>,
{
    type Item = f32;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        self.buf.resize(ctx.num_samples, 0.0);
        for (out, sample, room_size, damping, mix) in izip! {
            self.buf.iter_mut(),
            self.sig.sample(ctx).iter(),
            self.props.room_size.sample(ctx).iter(),
            self.props.damping.sample(ctx).iter(),
            self.props.mix.sample(ctx).iter(),
        } {
            if room_size != self.room_size_prev {
                self.room_size_prev = room_size;
                self.state.set_room_size(room_size as f64);
            }
            if damping != self.damping_prev {
                self.damping_prev = damping;
                self.state.set_damping(damping as f64);
            }
            let reverb = self.state.process(sample as f64) as f32;
            *out = (reverb * mix) + (sample * (1.0 - mix));
        }
        &self.buf
    }
}
