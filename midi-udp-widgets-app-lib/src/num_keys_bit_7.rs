use crate::{
    midi,
    widget::{ByTitle, MidiControllerBoolUdp, MidiControllerU7Udp, Widget},
};
use caw_core::{Sig, SigShared};
use caw_midi::MidiMessagesT;
use lazy_static::lazy_static;

pub type NumKeysBits7WithSpace = (
    Sig<SigShared<MidiControllerU7Udp>>,
    Sig<SigShared<MidiControllerBoolUdp>>,
);

lazy_static! {
    // Used to prevent multiple windows from opening at the same time with the same name. Note that
    // when using with evcxr this seems to get cleared when a cell is recomputed, but this is still
    // valuable in the context of stereo signals, where a function is evaluated once for the left
    // channel and once for the right channel. In such a case, this prevents each knob openned by
    // that function from being openned twice.
    static ref BY_TITLE: ByTitle<NumKeysBits7WithSpace> = Default::default();
}

fn new_num_keys_bits_7_with_space(title: String) -> NumKeysBits7WithSpace {
    BY_TITLE.get_or_insert(title.as_str(), || {
        let (channel, controller) = midi::alloc_controller();
        let (space_channel, space_controller) = midi::alloc_controller();
        let widget = Widget::new(
            title.clone(),
            channel,
            "num-keys-bits7",
            vec![
                format!("--controller={}", controller),
                format!("--space-channel={}", space_channel),
                format!("--space-controller={}", space_controller),
            ],
        )
        .unwrap();
        let controllers = widget.channel().controllers();
        let value = controllers.get_u7(controller.into()).shared();
        let space = controllers.get_bool(space_controller.into()).shared();
        (value, space)
    })
}

fn new_num_keys_bits_7(title: String) -> Sig<SigShared<MidiControllerU7Udp>> {
    new_num_keys_bits_7_with_space(title).0
}

mod builder {
    use super::*;
    use caw_builder_proc_macros::builder;
    use caw_core::{Sig, SigShared};

    builder! {
        #[constructor = "num_keys_bits_7_"]
        #[constructor_doc = "A 7-bit integer representing the state of the first 7 number keys"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "new_num_keys_bits_7"]
        #[build_ty = "Sig<SigShared<MidiControllerU7Udp>>"]
        pub struct Props {
            title: String,
        }
    }

    pub fn num_keys_bits_7(title: impl Into<String>) -> Props {
        num_keys_bits_7_(title.into())
    }

    builder! {
        #[constructor = "num_keys_bits_7_with_space_"]
        #[constructor_doc = "A 7-bit integer representing the state of the first 7 number keys"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "new_num_keys_bits_7_with_space"]
        #[build_ty = "NumKeysBits7WithSpace"]
        pub struct WithSpaceProps {
            title: String,
        }
    }

    pub fn num_keys_bits_7_with_space(
        title: impl Into<String>,
    ) -> WithSpaceProps {
        num_keys_bits_7_with_space_(title.into())
    }
}

pub use builder::{num_keys_bits_7, num_keys_bits_7_with_space};
