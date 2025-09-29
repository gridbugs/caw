use crate::{
    midi,
    widget::{ByTitle, MidiController01Udp, MidiControllerBoolUdp, Widget},
};
use caw_core::{Sig, SigShared};
use caw_midi::MidiMessagesT;
use lazy_static::lazy_static;

pub type KnobWithSpace = (
    Sig<SigShared<MidiController01Udp>>,
    Sig<SigShared<MidiControllerBoolUdp>>,
);

lazy_static! {
    // Used to prevent multiple windows from opening at the same time with the same name. Note that
    // when using with evcxr this seems to get cleared when a cell is recomputed, but this is still
    // valuable in the context of stereo signals, where a function is evaluated once for the left
    // channel and once for the right channel. In such a case, this prevents each knob openned by
    // that function from being openned twice.
    static ref BY_TITLE: ByTitle<KnobWithSpace> = Default::default();
}

fn new_knob_with_space(
    title: String,
    initial_value_01: f32,
    sensitivity: f32,
) -> KnobWithSpace {
    BY_TITLE.get_or_insert(title.as_str(), || {
        let channel = midi::alloc_channel();
        let controller = midi::alloc_controller(channel);
        let space_controller = midi::alloc_controller(channel);
        let widget = Widget::new(
            title.clone(),
            channel,
            "knob",
            vec![
                format!("--controller={}", controller),
                format!("--initial-value={}", initial_value_01),
                format!("--sensitivity={}", sensitivity),
                format!("--space-controller={}", space_controller),
            ],
        )
        .unwrap();
        let controllers = widget.channel().controllers();
        let value = controllers
            .get_with_initial_value_01(controller.into(), initial_value_01)
            .shared();
        let space = controllers.get_bool(space_controller.into()).shared();
        (value, space)
    })
}

fn new_knob(
    title: String,
    initial_value_01: f32,
    sensitivity: f32,
) -> Sig<SigShared<MidiController01Udp>> {
    new_knob_with_space(title, initial_value_01, sensitivity).0
}

mod builder {
    use super::*;
    use caw_builder_proc_macros::builder;
    use caw_core::{Sig, SigShared};

    builder! {
        #[constructor = "knob_"]
        #[constructor_doc = "A visual knob in a new window"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "new_knob"]
        #[build_ty = "Sig<SigShared<MidiController01Udp>>"]
        pub struct KnobProps {
            title: String,
            #[default = 0.5]
            initial_value_01: f32,
            #[default = 0.2]
            sensitivity: f32,
        }
    }

    pub fn knob(title: impl Into<String>) -> KnobProps {
        knob_(title.into())
    }

    builder! {
        #[constructor = "knob_with_space_"]
        #[constructor_doc = "A visual knob in a new window"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "new_knob_with_space"]
        #[build_ty = "KnobWithSpace"]
        pub struct KnobWithSpaceProps {
            title: String,
            #[default = 0.5]
            initial_value_01: f32,
            #[default = 0.2]
            sensitivity: f32,
        }
    }

    pub fn knob_with_space(title: impl Into<String>) -> KnobWithSpaceProps {
        knob_with_space_(title.into())
    }
}

pub use builder::{knob, knob_with_space};
