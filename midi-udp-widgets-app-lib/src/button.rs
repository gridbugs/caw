use crate::{
    midi,
    server::MidiChannelUdp,
    widget::{ByTitle, MidiControllerBoolUdp, Widget},
};
use caw_core::{Sig, SigShared, sig_shared};
use caw_keyboard::Note;
use caw_midi::{MidiKeyPress, MidiMessagesT};
use lazy_static::lazy_static;

pub type ButtonWithSpace = (
    Sig<SigShared<MidiKeyPress<MidiChannelUdp>>>,
    Sig<SigShared<MidiControllerBoolUdp>>,
);

lazy_static! {
    static ref BY_TITLE: ByTitle<ButtonWithSpace> = Default::default();
}

fn new_button_with_space(title: String) -> ButtonWithSpace {
    let (channel, key) = midi::alloc_key();
    let (space_channel, space_controller) = midi::alloc_controller();
    let note = Note::from_midi_index(key);
    BY_TITLE.get_or_insert(title.as_str(), || {
        let widget = Widget::new(
            title.clone(),
            channel,
            "button",
            vec![
                format!("--note={}", note),
                format!("--space-channel={}", space_channel),
                format!("--space-controller={}", space_controller),
            ],
        )
        .unwrap();
        let controllers = widget.channel().controllers();
        let value = sig_shared(widget.channel().key_press(note));
        let space = controllers.get_bool(space_controller.into()).shared();
        (value, space)
    })
}

fn new_button(title: String) -> Sig<SigShared<MidiKeyPress<MidiChannelUdp>>> {
    new_button_with_space(title).0
}

mod builder {
    use super::*;
    use caw_builder_proc_macros::builder;
    use caw_core::{Sig, SigShared};

    builder! {
        #[constructor = "button_"]
        #[constructor_doc = "A visual button in a new window"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "new_button"]
        #[build_ty = "Sig<SigShared<MidiKeyPress<MidiChannelUdp>>>"]
        pub struct Props {
            title: String,
        }
    }

    pub fn button(title: impl Into<String>) -> Props {
        button_(title.into())
    }

    builder! {
        #[constructor = "button_with_space_"]
        #[constructor_doc = "A visual button in a new window"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "new_button_with_space"]
        #[build_ty = "ButtonWithSpace"]
        pub struct PropsWithSpace {
            title: String,
        }
    }

    pub fn button_with_space(title: impl Into<String>) -> PropsWithSpace {
        button_with_space_(title.into())
    }
}

pub use builder::{button, button_with_space};
