use crate::{
    midi,
    server::MidiChannelUdp,
    widget::{ByTitle, Widget},
};
use caw_core::{Sig, SigShared, sig_shared};
use caw_keyboard::Note;
use caw_midi::{MidiKeyPress, MidiMessagesT};
use lazy_static::lazy_static;

pub type Switch = MidiKeyPress<MidiChannelUdp>;

lazy_static! {
    static ref BY_TITLE: ByTitle<Sig<SigShared<Switch>>> = Default::default();
}

fn new_switch(title: String) -> Sig<SigShared<Switch>> {
    let (channel, key) = midi::alloc_key();
    let note = Note::from_midi_index(key);
    BY_TITLE.get_or_insert(title.as_str(), || {
        let widget = Widget::new(
            title.clone(),
            channel,
            "switch",
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
        #[constructor = "switch_"]
        #[constructor_doc = "A visual switch in a new window"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "new_switch"]
        #[build_ty = "Sig<SigShared<Switch>>"]
        pub struct Props {
            title: String,
        }
    }

    pub fn switch(title: impl Into<String>) -> Props {
        switch_(title.into())
    }
}

pub use builder::switch;
