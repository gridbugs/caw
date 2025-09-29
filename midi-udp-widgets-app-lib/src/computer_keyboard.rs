use crate::{
    midi,
    server::MidiChannelUdp,
    widget::{ByTitle, Widget},
};
use caw_core::{Sig, SigShared, SigT};
use caw_keyboard::{KeyEvents, Note};
use caw_midi::MidiMessagesT;
use lazy_static::lazy_static;

lazy_static! {
    static ref BY_TITLE: ByTitle<Sig<SigShared<MidiChannelUdp>>> =
        Default::default();
}

fn new_computer_keyboard(
    title: String,
    start_note: Note,
) -> Sig<SigShared<MidiChannelUdp>> {
    BY_TITLE.get_or_insert(title.as_str(), || {
        let channel = midi::alloc_channel();
        let widget = Widget::new(
            title.clone(),
            channel,
            "computer-keyboard",
            vec![format!("--start-note={}", start_note)],
        )
        .unwrap();
        widget.channel().shared()
    })
}

pub struct ComputerKeyboard {
    sig: Sig<SigShared<MidiChannelUdp>>,
}

impl ComputerKeyboard {
    pub fn key_events(&self) -> Sig<impl SigT<Item = KeyEvents> + use<>> {
        self.sig.clone().key_events()
    }

    pub fn space_button(&self) -> Sig<impl SigT<Item = bool> + use<>> {
        self.sig.clone().controllers().get_01(0).is_positive()
    }
}

mod builder {
    use super::*;
    use caw_builder_proc_macros::builder;
    use caw_core::{Sig, SigShared};
    use caw_keyboard::Note;

    builder! {
        #[constructor = "computer_keyboard_"]
        #[constructor_doc = "Generate midi events with a computer keyboard when this widget is focused"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "new_computer_keyboard"]
        #[build_ty = "Sig<SigShared<MidiChannelUdp>>"]
        pub struct Props {
            title: String,
            #[default = Note::B_1]
            start_note: Note,
        }
    }

    impl Props {
        /// Like `build` but returns a higher level object rather than a midi stream.
        pub fn build_(self) -> ComputerKeyboard {
            ComputerKeyboard { sig: self.build() }
        }
    }

    pub fn computer_keyboard(title: impl Into<String>) -> Props {
        computer_keyboard_(title.into())
    }
}

pub use builder::computer_keyboard;
