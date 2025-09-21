use crate::{
    midi,
    server::MidiChannelUdp,
    widget::{ByTitle, Widget},
};
use caw_core::{Buf, Sig, SigShared, SigT};
use caw_keyboard::Note;
use caw_midi::MidiMessages;
use lazy_static::lazy_static;

pub struct ComputerKeyboard {
    sig: Sig<MidiChannelUdp>,
}

lazy_static! {
    static ref BY_TITLE: ByTitle<ComputerKeyboard> = Default::default();
}

impl ComputerKeyboard {
    pub fn new(title: String, start_note: Note) -> Sig<SigShared<Self>> {
        BY_TITLE.get_or_insert(title.as_str(), || {
            let channel = midi::alloc_channel();
            let widget = Widget::new(
                title.clone(),
                channel,
                "computer-keyboard",
                vec![format!("--start-note={}", start_note)],
            )
            .unwrap();
            let sig = widget.channel();
            Self { sig }
        })
    }
}

impl SigT for ComputerKeyboard {
    type Item = MidiMessages;

    fn sample(&mut self, ctx: &caw_core::SigCtx) -> impl Buf<Self::Item> {
        self.sig.sample(ctx)
    }
}

mod computer_keyboard_builder {
    use super::ComputerKeyboard;
    use caw_builder_proc_macros::builder;
    use caw_core::{Sig, SigShared};
    use caw_keyboard::Note;

    builder! {
        #[constructor = "computer_keyboard_"]
        #[constructor_doc = "Generate midi events with a computer keyboard when this widget is focused"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "ComputerKeyboard::new"]
        #[build_ty = "Sig<SigShared<ComputerKeyboard>>"]
        pub struct Props {
            title: String,
            #[default = Note::B_1]
            start_note: Note,
        }
    }

    pub fn computer_keyboard(title: impl Into<String>) -> Props {
        computer_keyboard_(title.into())
    }
}

pub use computer_keyboard_builder::computer_keyboard;
