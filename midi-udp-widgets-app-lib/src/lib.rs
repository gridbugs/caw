use caw_core::{Buf, Sig, SigShared, SigT, Zip, sig_shared};
use caw_keyboard::Note;
use caw_midi::{
    MidiChannel, MidiController01, MidiEventsT, MidiKeyPress, MidiMessages,
    MidiMessagesT,
};
use caw_midi_udp::MidiLiveUdp;
use lazy_static::lazy_static;
use midly::num::{u4, u7};
use std::{
    collections::HashMap,
    net::SocketAddr,
    process::{Child, Command},
    sync::Mutex,
};

struct ChannelAllocator {
    next_channel: Option<u4>,
    next_controller_index_by_channel: HashMap<u4, Option<u7>>,
}

impl ChannelAllocator {
    fn new() -> Self {
        Self {
            next_channel: Some(0.into()),
            next_controller_index_by_channel: HashMap::new(),
        }
    }

    fn alloc_channel(&mut self) -> u4 {
        let channel = self.next_channel.expect("Can't allocate MIDI channel because all MIDI channels are in use already.");
        self.next_channel = if channel == u4::max_value() {
            None
        } else {
            Some((u8::from(channel) + 1).into())
        };
        channel
    }

    fn alloc_controller(&mut self, channel: u4) -> u7 {
        if let Some(next_channel) = self.next_channel {
            assert!(
                channel < next_channel,
                "Attempted to allocate controller on unallocated channel."
            );
        }
        let next_controller = self
            .next_controller_index_by_channel
            .entry(channel)
            .or_insert_with(|| Some(0.into()));
        if let Some(controller) = *next_controller {
            *next_controller = if controller == u7::max_value() {
                None
            } else {
                Some((u8::from(controller) + 1).into())
            };
            controller
        } else {
            panic!(
                "Can't allocate MIDI controller because all MIDI controllers on channel {} are in use already.",
                channel
            );
        }
    }
}

lazy_static! {
    // A stream of MIDI events received by the UDP/IP server. It owns the UDP/IP server.
    static ref SIG: Sig<SigShared<MidiLiveUdp>> =
        sig_shared(MidiLiveUdp::new_unspecified().unwrap());

    // Used to prevent multiple windows from opening at the same time with the same name. Note that
    // when using with evcxr this seems to get cleared when a cell is recomputed, but this is still
    // valuable in the context of stereo signals, where a function is evaluated once for the left
    // channel and once for the right channel. In such a case, this prevents each knob openned by
    // that function from being openned twice.
    static ref BUTTONS_BY_TITLE: Mutex<HashMap<String, Sig<SigShared<Button>>>> =
        Mutex::new(HashMap::new());
    static ref KNOBS_BY_TITLE: Mutex<HashMap<String, Sig<SigShared<Knob>>>> =
        Mutex::new(HashMap::new());
    static ref XYS_BY_TITLE: Mutex<HashMap<String, Sig<SigShared<Xy>>>> =
        Mutex::new(HashMap::new());
    static ref COMPUTER_KEYBOARDS_BY_TITLE: Mutex<HashMap<String, Sig<SigShared<ComputerKeyboard>>>> =
        Mutex::new(HashMap::new());

    static ref CHANNEL_ALLOCATOR: Mutex<ChannelAllocator> = Mutex::new(ChannelAllocator::new());
}

/// Returns the IP address of the server that will receive MIDI events.
fn sig_server_local_socket_address() -> SocketAddr {
    SIG.0.with_inner(|midi_live_udp| {
        midi_live_udp.local_socket_address().unwrap()
    })
}

const PROGRAM_NAME: &str = "caw_midi_udp_widgets_app";

type MidiChannelUdp = MidiChannel<SigShared<MidiLiveUdp>>;

struct Widget {
    title: String,
    process: Option<Child>,
}

pub struct Button {
    widget: Widget,
    sig: Sig<MidiKeyPress<MidiChannelUdp>>,
    channel_index: u4,
}

impl Button {
    pub fn new(title: String) -> Sig<SigShared<Self>> {
        let mut buttons_by_title = BUTTONS_BY_TITLE.lock().unwrap();
        if let Some(button) = buttons_by_title.get(&title) {
            button.clone()
        } else {
            let channel_index =
                CHANNEL_ALLOCATOR.lock().unwrap().alloc_channel();
            let channel = SIG.clone().channel(channel_index.into());
            // The choice of note here is arbitrary.
            let sig = Sig(channel.key_press(Note::default()));
            let mut s = Self {
                widget: Widget {
                    title: title.clone(),
                    process: None,
                },
                sig,
                channel_index,
            };
            let child = match s.command().spawn() {
                Ok(child) => child,
                Err(e) => panic!(
                    "{} (make sure `{}` is in your PATH",
                    e, PROGRAM_NAME
                ),
            };
            s.widget.process = Some(child);
            let s = sig_shared(s);
            buttons_by_title.insert(title, s.clone());
            s
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(PROGRAM_NAME);
        let args = vec![
            format!(
                "--server={}",
                sig_server_local_socket_address().to_string()
            ),
            format!("--channel={}", self.channel_index),
            format!("--title={}", self.widget.title.clone()),
            "button".to_string(),
        ];
        command.args(args);
        command
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

pub struct Knob {
    widget: Widget,
    controller: u8,
    initial_value_01: f32,
    sensitivity: f32,
    sig: Sig<MidiController01<MidiChannelUdp>>,
    channel_index: u4,
}

impl Knob {
    pub fn new(
        title: String,
        initial_value_01: f32,
        sensitivity: f32,
    ) -> Sig<SigShared<Self>> {
        let mut knobs_by_title = KNOBS_BY_TITLE.lock().unwrap();
        if let Some(knob) = knobs_by_title.get(&title) {
            knob.clone()
        } else {
            let mut allocator = CHANNEL_ALLOCATOR.lock().unwrap();
            let channel_index = allocator.alloc_channel();
            let controller = allocator.alloc_controller(channel_index).into();
            let channel = SIG.clone().channel(channel_index.into());
            let sig = channel
                .controllers()
                .get_with_initial_value_01(controller, initial_value_01);
            let mut s = Self {
                widget: Widget {
                    title: title.clone(),
                    process: None,
                },
                controller,
                initial_value_01,
                sensitivity,
                sig,
                channel_index,
            };
            let child = match s.command().spawn() {
                Ok(child) => child,
                Err(e) => panic!(
                    "{} (make sure `{}` is in your PATH",
                    e, PROGRAM_NAME
                ),
            };
            s.widget.process = Some(child);
            let s = sig_shared(s);
            knobs_by_title.insert(title, s.clone());
            s
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(PROGRAM_NAME);
        let args = vec![
            format!(
                "--server={}",
                sig_server_local_socket_address().to_string()
            ),
            format!("--channel={}", self.channel_index),
            format!("--title={}", self.widget.title.clone()),
            "knob".to_string(),
            format!("--controller={}", self.controller),
            format!("--initial-value={}", self.initial_value_01),
            format!("--sensitivity={}", self.sensitivity),
        ];
        command.args(args);
        command
    }
}

impl SigT for Knob {
    type Item = f32;

    fn sample(&mut self, ctx: &caw_core::SigCtx) -> impl Buf<Self::Item> {
        self.sig.sample(ctx)
    }
}

mod knob_builder {
    use super::Knob;
    use caw_builder_proc_macros::builder;
    use caw_core::{Sig, SigShared};

    builder! {
        #[constructor = "knob_"]
        #[constructor_doc = "A visual knob in a new window"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "Knob::new"]
        #[build_ty = "Sig<SigShared<Knob>>"]
        pub struct Props {
            title: String,
            #[default = 0.5]
            initial_value_01: f32,
            #[default = 0.2]
            sensitivity: f32,
        }
    }

    pub fn knob(title: impl Into<String>) -> Props {
        knob_(title.into())
    }
}

pub use knob_builder::knob;

pub struct Xy {
    widget: Widget,
    controller_x: u8,
    controller_y: u8,
    axis_label_x: Option<String>,
    axis_label_y: Option<String>,
    sig: Sig<
        Zip<
            MidiController01<SigShared<MidiChannelUdp>>,
            MidiController01<SigShared<MidiChannelUdp>>,
        >,
    >,
    channel_index: u4,
}

impl Xy {
    pub fn new(
        title: String,
        axis_label_x: Option<String>,
        axis_label_y: Option<String>,
    ) -> Sig<SigShared<Self>> {
        let mut xys_by_title = XYS_BY_TITLE.lock().unwrap();
        if let Some(xy) = xys_by_title.get(&title) {
            xy.clone()
        } else {
            let mut allocator = CHANNEL_ALLOCATOR.lock().unwrap();
            let channel_index = allocator.alloc_channel();
            let controller_x = allocator.alloc_controller(channel_index).into();
            let controller_y = allocator.alloc_controller(channel_index).into();
            let channel = SIG.clone().channel(channel_index.into()).shared();
            let sig_x = channel
                .clone()
                .controllers()
                .get_with_initial_value_01(controller_x, 0.5);
            let sig_y = channel
                .clone()
                .controllers()
                .get_with_initial_value_01(controller_y, 0.5);
            let sig = sig_x.zip(sig_y.0);
            let mut s = Self {
                widget: Widget {
                    title: title.clone(),
                    process: None,
                },
                controller_x,
                controller_y,
                sig,
                channel_index,
                axis_label_x,
                axis_label_y,
            };
            let child = match s.command().spawn() {
                Ok(child) => child,
                Err(e) => panic!(
                    "{} (make sure `{}` is in your PATH",
                    e, PROGRAM_NAME
                ),
            };
            s.widget.process = Some(child);
            let s = sig_shared(s);
            xys_by_title.insert(title, s.clone());
            s
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(PROGRAM_NAME);
        let mut args = vec![
            format!(
                "--server={}",
                sig_server_local_socket_address().to_string()
            ),
            format!("--channel={}", self.channel_index),
            format!("--title={}", self.widget.title.clone()),
            "xy".to_string(),
            format!("--controller-x={}", self.controller_x),
            format!("--controller-y={}", self.controller_y),
        ];
        if let Some(axis_label_x) = self.axis_label_x.as_ref() {
            args.push(format!("--axis-label-x={}", axis_label_x));
        }
        if let Some(axis_label_y) = self.axis_label_y.as_ref() {
            args.push(format!("--axis-label-y={}", axis_label_y));
        }
        command.args(args);
        command
    }
}

impl SigT for Xy {
    type Item = (f32, f32);

    fn sample(&mut self, ctx: &caw_core::SigCtx) -> impl Buf<Self::Item> {
        self.sig.sample(ctx)
    }
}

mod xy_builder {
    use super::Xy;
    use caw_builder_proc_macros::builder;
    use caw_core::{Sig, SigShared};

    builder! {
        #[constructor = "xy_"]
        #[constructor_doc = "A visual XY planen in a new window"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "Xy::new"]
        #[build_ty = "Sig<SigShared<Xy>>"]
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

pub struct ComputerKeyboard {
    widget: Widget,
    start_note: Note,
    sig: Sig<MidiChannelUdp>,
    channel_index: u4,
}

impl ComputerKeyboard {
    pub fn new(title: String, start_note: Note) -> Sig<SigShared<Self>> {
        let mut computer_keyboads_by_title =
            COMPUTER_KEYBOARDS_BY_TITLE.lock().unwrap();
        if let Some(computer_keyboard) = computer_keyboads_by_title.get(&title)
        {
            computer_keyboard.clone()
        } else {
            let mut allocator = CHANNEL_ALLOCATOR.lock().unwrap();
            let channel_index = allocator.alloc_channel();
            let sig = SIG.clone().channel(channel_index.into());
            let mut s = Self {
                widget: Widget {
                    title: title.clone(),
                    process: None,
                },
                start_note,
                sig,
                channel_index,
            };
            let child = match s.command().spawn() {
                Ok(child) => child,
                Err(e) => panic!(
                    "{} (make sure `{}` is in your PATH",
                    e, PROGRAM_NAME
                ),
            };
            s.widget.process = Some(child);
            let s = sig_shared(s);
            computer_keyboads_by_title.insert(title, s.clone());
            s
        }
    }

    fn command(&self) -> Command {
        let mut command = Command::new(PROGRAM_NAME);
        let args = vec![
            format!(
                "--server={}",
                sig_server_local_socket_address().to_string()
            ),
            format!("--channel={}", self.channel_index),
            format!("--title={}", self.widget.title.clone()),
            "computer-keyboard".to_string(),
            format!("--start-note={}", self.start_note),
        ];
        command.args(args);
        command
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
