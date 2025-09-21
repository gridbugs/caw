use crate::{
    midi,
    server::MidiChannelUdp,
    widget::{ByTitle, Widget},
};
use caw_core::{Buf, Sig, SigShared, SigT};
use caw_keyboard::Note;
use caw_midi::{MidiKeyPress, MidiMessagesT};
use lazy_static::lazy_static;

pub struct Button {
    sig: Sig<MidiKeyPress<MidiChannelUdp>>,
}

lazy_static! {
    static ref BY_TITLE: ByTitle<Button> = Default::default();
}

impl Button {
    pub fn new(title: String) -> Sig<SigShared<Self>> {
        BY_TITLE.get_or_insert(title.as_str(), || {
            let widget = Widget::new(
                title.clone(),
                midi::alloc_channel(),
                "button",
                vec![],
            )
            .unwrap();
            // The choice of note here is arbitrary.
            let sig = Sig(widget.channel().key_press(Note::default()));
            Self { sig }
        })
    }
}

impl SigT for Button {
    type Item = bool;

    fn sample(&mut self, ctx: &caw_core::SigCtx) -> impl Buf<Self::Item> {
        self.sig.sample(ctx)
    }
}

mod button_builder {
    use super::Button;
    use caw_builder_proc_macros::builder;
    use caw_core::{Sig, SigShared};

    builder! {
        #[constructor = "button_"]
        #[constructor_doc = "A visual button in a new window"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "Button::new"]
        #[build_ty = "Sig<SigShared<Button>>"]
        pub struct Props {
            title: String,
        }
    }

    pub fn button(title: impl Into<String>) -> Props {
        button_(title.into())
    }
}

pub use button_builder::button;
