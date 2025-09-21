use crate::{
    midi,
    widget::{ByTitle, MidiController01Udp, Widget},
};
use caw_core::{IsPositive, Sig, SigShared};
use caw_midi::MidiMessagesT;
use lazy_static::lazy_static;

pub type XyPairShared = (
    Sig<SigShared<MidiController01Udp>>,
    Sig<SigShared<MidiController01Udp>>,
);

pub type XyWithSpace = (
    Sig<SigShared<MidiController01Udp>>,
    Sig<SigShared<MidiController01Udp>>,
    Sig<SigShared<IsPositive<MidiController01Udp>>>,
);
lazy_static! {
    static ref BY_TITLE: ByTitle<XyWithSpace> = Default::default();
}

fn new_xy_with_space(
    title: String,
    axis_label_x: Option<String>,
    axis_label_y: Option<String>,
) -> XyWithSpace {
    BY_TITLE.get_or_insert(title.as_str(), || {
        let channel = midi::alloc_channel();
        let controller_x = midi::alloc_controller(channel);
        let controller_y = midi::alloc_controller(channel);
        let space_controller = midi::alloc_controller(channel);
        let mut args = vec![
            format!("--controller-x={}", controller_x),
            format!("--controller-y={}", controller_y),
            format!("--space-controller={}", space_controller),
        ];
        if let Some(axis_label_x) = axis_label_x.as_ref() {
            args.push(format!("--axis-label-x={}", axis_label_x));
        }
        if let Some(axis_label_y) = axis_label_y.as_ref() {
            args.push(format!("--axis-label-y={}", axis_label_y));
        }
        let widget = Widget::new(title.clone(), channel, "xy", args).unwrap();
        let controllers = widget.channel().controllers();
        let sig_x = controllers
            .get_with_initial_value_01(controller_x.into(), 0.5)
            .shared();
        let sig_y = controllers
            .get_with_initial_value_01(controller_y.into(), 0.5)
            .shared();
        let space = controllers
            .get_with_initial_value_01(space_controller.into(), 0.0)
            .is_positive()
            .shared();
        (sig_x, sig_y, space)
    })
}

fn new_xy(
    title: String,
    axis_label_x: Option<String>,
    axis_label_y: Option<String>,
) -> XyPairShared {
    let (x, y, _) = new_xy_with_space(title, axis_label_x, axis_label_y);
    (x, y)
}

mod builder {
    use super::*;
    use caw_builder_proc_macros::builder;

    builder! {
        #[constructor = "xy_"]
        #[constructor_doc = "A visual XY planen in a new window"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "new_xy"]
        #[build_ty = "XyPairShared"]
        pub struct XyProps {
            title: String,
            #[default = None]
            axis_label_x_: Option<String>,
            #[default = None]
            axis_label_y_: Option<String>,
        }
    }

    impl XyProps {
        pub fn axis_label_x(self, axis_label_x: impl AsRef<str>) -> Self {
            self.axis_label_x_(Some(axis_label_x.as_ref().to_string()))
        }
        pub fn axis_label_y(self, axis_label_y: impl AsRef<str>) -> Self {
            self.axis_label_y_(Some(axis_label_y.as_ref().to_string()))
        }
    }

    pub fn xy(title: impl Into<String>) -> XyProps {
        xy_(title.into())
    }

    builder! {
        #[constructor = "xy_with_space_"]
        #[constructor_doc = "A visual XY planen in a new window"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "new_xy_with_space"]
        #[build_ty = "XyWithSpace"]
        pub struct XyWithSpaceProps {
            title: String,
            #[default = None]
            axis_label_x_: Option<String>,
            #[default = None]
            axis_label_y_: Option<String>,
        }
    }

    impl XyWithSpaceProps {
        pub fn axis_label_x(self, axis_label_x: impl AsRef<str>) -> Self {
            self.axis_label_x_(Some(axis_label_x.as_ref().to_string()))
        }
        pub fn axis_label_y(self, axis_label_y: impl AsRef<str>) -> Self {
            self.axis_label_y_(Some(axis_label_y.as_ref().to_string()))
        }
    }

    pub fn xy_with_space(title: impl Into<String>) -> XyWithSpaceProps {
        xy_with_space_(title.into())
    }
}

pub use builder::{xy, xy_with_space};
