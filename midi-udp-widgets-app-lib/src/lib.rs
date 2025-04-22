use caw_core::{Buf, Sig, SigShared, SigT, sig_shared};
use caw_midi::{MidiController01, MidiMessagesT};
use caw_midi_udp::{MidiLiveUdp, MidiLiveUdpChannel};
use lazy_static::lazy_static;
use std::{
    net::SocketAddr,
    process::{Child, Command},
};

// For now this library only supports midi channel 0
const CHANNEL: u8 = 0;

lazy_static! {
    static ref SIG: Sig<SigShared<MidiLiveUdpChannel>> =
        sig_shared(MidiLiveUdp::new_unspecified().unwrap().channel(CHANNEL).0);
}

fn sig_server_local_socket_address() -> SocketAddr {
    SIG.0.with_inner(|midi_live_udp_channel| {
        midi_live_udp_channel.server.local_socket_address().unwrap()
    })
}

const PROGRAM_NAME: &str = "caw_midi_udp_widgets_app";

pub struct Knob {
    controller: u8,
    title: Option<String>,
    initial_value_01: f32,
    sensitivity: f32,
    process: Option<Child>,
    sig: Sig<MidiController01<SigShared<MidiLiveUdpChannel>>>,
}

impl Knob {
    pub fn new(
        controller: u8,
        title: Option<String>,
        initial_value_01: f32,
        sensitivity: f32,
    ) -> Sig<Self> {
        let sig = SIG
            .clone()
            .controllers()
            .get_with_initial_value_01(controller, initial_value_01);
        let mut s = Self {
            controller,
            title,
            initial_value_01,
            sensitivity,
            process: None,
            sig,
        };
        let child = s
            .command()
            .unwrap()
            .spawn()
            .expect("Failed to launch process");
        s.process = Some(child);
        Sig(s)
    }

    fn command(&self) -> anyhow::Result<Command> {
        let mut command = Command::new(PROGRAM_NAME);
        let mut args = vec![
            "knob".to_string(),
            "--server".to_string(),
            sig_server_local_socket_address().to_string(),
            "--channel".to_string(),
            format!("{}", CHANNEL),
            "--controller".to_string(),
            format!("{}", self.controller),
            "--initial-value".to_string(),
            format!("{}", self.initial_value_01),
            "--sensitivity".to_string(),
            format!("{}", self.sensitivity),
        ];
        if let Some(title) = self.title.as_ref() {
            args.extend(["--title".to_string(), title.clone()]);
        }
        command.args(args);
        Ok(command)
    }
}

impl SigT for Knob {
    type Item = f32;

    fn sample(&mut self, ctx: &caw_core::SigCtx) -> impl Buf<Self::Item> {
        self.sig.sample(ctx)
    }
}

mod builder {
    use super::Knob;
    use caw_builder_proc_macros::builder;
    use caw_core::Sig;

    builder! {
        #[constructor = "knob"]
        #[constructor_doc = "A visual knob in a new window"]
        #[generic_setter_type_name = "X"]
        #[build_fn = "Knob::new"]
        #[build_ty = "Sig<Knob>"]
        pub struct Props {
            #[default = 0]
            controller: u8,
            #[default = None]
            title_opt: Option<String>,
            #[default = 0.5]
            initial_value_01: f32,
            #[default = 0.2]
            sensitivity: f32,
        }
    }

    impl Props {
        pub fn title(self, title: impl Into<String>) -> Self {
            self.title_opt(Some(title.into()))
        }
    }
}

pub use builder::knob;
