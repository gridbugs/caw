use crate::{
    midi,
    server::MidiChannelUdp,
    widget::{ByTitle, Widget},
};
use caw_core::{Buf, Sig, SigShared, SigT};
use caw_midi::{MidiController01, MidiMessagesT};
use lazy_static::lazy_static;

pub struct Knob {
    sig: Sig<MidiController01<MidiChannelUdp>>,
}

lazy_static! {
    // Used to prevent multiple windows from opening at the same time with the same name. Note that
    // when using with evcxr this seems to get cleared when a cell is recomputed, but this is still
    // valuable in the context of stereo signals, where a function is evaluated once for the left
    // channel and once for the right channel. In such a case, this prevents each knob openned by
    // that function from being openned twice.
    static ref BY_TITLE: ByTitle<Knob> = Default::default();
}

impl Knob {
    pub fn new(
        title: String,
        initial_value_01: f32,
        sensitivity: f32,
    ) -> Sig<SigShared<Self>> {
        BY_TITLE.get_or_insert(title.as_str(), || {
            let channel = midi::alloc_channel();
            let controller = midi::alloc_controller(channel);
            let widget = Widget::new(
                title.clone(),
                channel,
                "knob",
                vec![
                    format!("--controller={}", controller),
                    format!("--initial-value={}", initial_value_01),
                    format!("--sensitivity={}", sensitivity),
                ],
            )
            .unwrap();
            let sig = widget
                .channel()
                .controllers()
                .get_with_initial_value_01(controller.into(), initial_value_01);
            Self { sig }
        })
    }
}

impl SigT for Knob {
    type Item = f32;

    fn sample(&mut self, ctx: &caw_core::SigCtx) -> impl Buf<Self::Item> {
        self.sig.sample(ctx)
    }
}

mod knob_builder {
    use super::Knob;
    use caw_builder_proc_macros::builder;
    use caw_core::{Sig, SigShared};

    builder! {
        #[constructor = "knob_"]
        #[constructor_doc = "A visual knob in a new window"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "Knob::new"]
        #[build_ty = "Sig<SigShared<Knob>>"]
        pub struct Props {
            title: String,
            #[default = 0.5]
            initial_value_01: f32,
            #[default = 0.2]
            sensitivity: f32,
        }
    }

    pub fn knob(title: impl Into<String>) -> Props {
        knob_(title.into())
    }
}

pub use knob_builder::knob;
