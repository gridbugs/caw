use crate::{
    midi,
    server::MidiChannelUdp,
    widget::{ByTitle, Widget},
};
use caw_core::{Sig, SigShared};
use caw_midi::{MidiController01, MidiMessagesT};
use lazy_static::lazy_static;

pub type XySingle = MidiController01<SigShared<MidiChannelUdp>>;
pub type XyPairShared = (Sig<SigShared<XySingle>>, Sig<SigShared<XySingle>>);

lazy_static! {
    static ref BY_TITLE: ByTitle<XyPairShared> = Default::default();
}

fn new_xy(
    title: String,
    axis_label_x: Option<String>,
    axis_label_y: Option<String>,
) -> XyPairShared {
    BY_TITLE.get_or_insert(title.as_str(), || {
        let channel = midi::alloc_channel();
        let controller_x = midi::alloc_controller(channel);
        let controller_y = midi::alloc_controller(channel);
        let mut args = vec![
            format!("--controller-x={}", controller_x),
            format!("--controller-y={}", controller_y),
        ];
        if let Some(axis_label_x) = axis_label_x.as_ref() {
            args.push(format!("--axis-label-x={}", axis_label_x));
        }
        if let Some(axis_label_y) = axis_label_y.as_ref() {
            args.push(format!("--axis-label-y={}", axis_label_y));
        }
        let widget = Widget::new(title.clone(), channel, "xy", args).unwrap();
        let sig = widget.channel().shared();
        let sig_x = sig
            .clone()
            .controllers()
            .get_with_initial_value_01(controller_x.into(), 0.5)
            .shared();
        let sig_y = sig
            .clone()
            .controllers()
            .get_with_initial_value_01(controller_y.into(), 0.5)
            .shared();
        (sig_x, sig_y)
    })
}

mod xy_builder {
    use super::*;
    use caw_builder_proc_macros::builder;

    builder! {
        #[constructor = "xy_"]
        #[constructor_doc = "A visual XY planen in a new window"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "new_xy"]
        #[build_ty = "XyPairShared"]
        pub struct Props {
            title: String,
            #[default = None]
            axis_label_x_: Option<String>,
            #[default = None]
            axis_label_y_: Option<String>,
        }
    }

    impl Props {
        pub fn axis_label_x(self, axis_label_x: impl AsRef<str>) -> Self {
            self.axis_label_x_(Some(axis_label_x.as_ref().to_string()))
        }
        pub fn axis_label_y(self, axis_label_y: impl AsRef<str>) -> Self {
            self.axis_label_y_(Some(axis_label_y.as_ref().to_string()))
        }
    }

    pub fn xy(title: impl Into<String>) -> Props {
        xy_(title.into())
    }
}

pub use xy_builder::xy;
