use caw_core::{Buf, Sig, SigShared, SigT, Zip, sig_shared};
use caw_keyboard::Note;
use caw_midi::{MidiController01, MidiKeyPress, MidiMessagesT};
use caw_midi_udp::{MidiLiveUdp, MidiLiveUdpChannel};
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    net::SocketAddr,
    process::{Child, Command},
    sync::Mutex,
};

// For now this library only supports midi channel 0
const CHANNEL: u8 = 0;

lazy_static! {
    // A stream of MIDI events received by the UDP/IP server. It owns the UDP/IP server.
    static ref SIG: Sig<SigShared<MidiLiveUdpChannel>> =
        sig_shared(MidiLiveUdp::new_unspecified().unwrap().channel(CHANNEL).0);

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

    /// MIDI controller indices are allocated dynamically and this global tracks the value of the
    /// next controller index to allocate.
    static ref NEXT_CONTROLLER_INDEX: Mutex<u8> = Mutex::new(0);
}

/// Returns the IP address of the server that will receive MIDI events.
fn sig_server_local_socket_address() -> SocketAddr {
    SIG.0.with_inner(|midi_live_udp_channel| {
        midi_live_udp_channel.server.local_socket_address().unwrap()
    })
}

/// Allocates and returns unique MIDI controller value.
fn alloc_controller() -> u8 {
    let mut next_controller_index = NEXT_CONTROLLER_INDEX.lock().unwrap();
    let controller_index = *next_controller_index;
    *next_controller_index += 1;
    controller_index
}

const PROGRAM_NAME: &str = "caw_midi_udp_widgets_app";

struct Widget {
    title: String,
    process: Option<Child>,
}

pub struct Button {
    widget: Widget,
    sig: Sig<MidiKeyPress<SigShared<MidiLiveUdpChannel>>>,
}

impl Button {
    pub fn new(title: String) -> Sig<SigShared<Self>> {
        let mut buttons_by_title = BUTTONS_BY_TITLE.lock().unwrap();
        if let Some(button) = buttons_by_title.get(&title) {
            button.clone()
        } else {
            // The choice of note here is arbitrary.
            let sig = Sig(SIG.clone().key_press(Note::C4));
            let mut s = Self {
                widget: Widget {
                    title: title.clone(),
                    process: None,
                },
                sig,
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
            format!("--channel={}", CHANNEL),
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
    sig: Sig<MidiController01<SigShared<MidiLiveUdpChannel>>>,
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
            let controller = alloc_controller();
            let sig = SIG
                .clone()
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
            format!("--channel={}", CHANNEL),
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
    sig: Sig<
        Zip<
            MidiController01<SigShared<MidiLiveUdpChannel>>,
            MidiController01<SigShared<MidiLiveUdpChannel>>,
        >,
    >,
}

impl Xy {
    pub fn new(title: String) -> Sig<SigShared<Self>> {
        let mut xys_by_title = XYS_BY_TITLE.lock().unwrap();
        if let Some(xy) = xys_by_title.get(&title) {
            xy.clone()
        } else {
            let controller_x = alloc_controller();
            let controller_y = alloc_controller();
            let sig_x = SIG
                .clone()
                .controllers()
                .get_with_initial_value_01(controller_x, 0.5);
            let sig_y = SIG
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
        let args = vec![
            format!(
                "--server={}",
                sig_server_local_socket_address().to_string()
            ),
            format!("--channel={}", CHANNEL),
            format!("--title={}", self.widget.title.clone()),
            "xy".to_string(),
            format!("--controller-x={}", self.controller_x),
            format!("--controller-y={}", self.controller_y),
        ];
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
        }
    }

    pub fn xy(title: impl Into<String>) -> Props {
        xy_(title.into())
    }
}

pub use xy_builder::xy;
