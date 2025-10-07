use crate::{
    midi,
    server::MidiChannelUdp,
    widget::{ByTitle, Widget},
};
use caw_core::{Sig, SigShared, sig_shared};
use caw_keyboard::Note;
use caw_midi::{MidiKeyPress, MidiMessagesT};
use lazy_static::lazy_static;

pub type Button = MidiKeyPress<MidiChannelUdp>;

lazy_static! {
    static ref BY_TITLE: ByTitle<Sig<SigShared<Button>>> = Default::default();
}

fn new_button(title: String) -> Sig<SigShared<Button>> {
    let (channel, key) = midi::alloc_key();
    let note = Note::from_midi_index(key);
    BY_TITLE.get_or_insert(title.as_str(), || {
        let widget = Widget::new(
            title.clone(),
            channel,
            "button",
            vec![format!("--note={}", note)],
        )
        .unwrap();
        // The choice of note here is arbitrary.
        sig_shared(widget.channel().key_press(note))
    })
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
        #[build_ty = "Sig<SigShared<Button>>"]
        pub struct Props {
            title: String,
        }
    }

    pub fn button(title: impl Into<String>) -> Props {
        button_(title.into())
    }
}

pub use builder::button;
