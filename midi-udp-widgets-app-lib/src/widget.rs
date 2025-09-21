use crate::server::{self, MidiChannelUdp};
use caw_core::Sig;
use caw_midi::MidiController01;
use midly::num::u4;
use std::{collections::HashMap, process::Command, sync::Mutex};

const PROGRAM_NAME: &str = "caw_midi_udp_widgets_app";

pub struct Widget {
    channel_index: u4,
}

impl Widget {
    pub fn new(
        title: String,
        channel_index: u4,
        subcommand: impl AsRef<str>,
        mut subcommand_args: Vec<String>,
    ) -> anyhow::Result<Self> {
        let mut command = Command::new(PROGRAM_NAME);
        let subcommand = subcommand.as_ref().to_string();
        let mut args = vec![
            format!("--server={}", server::local_socket_address().to_string()),
            format!("--channel={}", channel_index),
            format!("--title={}", title.clone()),
            subcommand.clone(),
        ];
        args.append(&mut subcommand_args);
        command.args(args);
        log::info!(
            "Launching subprocess for {subcommand} \"{title}\" on channel {channel_index}"
        );
        match command.spawn() {
            Ok(_child) => (),
            Err(e) => anyhow::bail!(
                "{} (make sure `{}` is in your PATH",
                e,
                PROGRAM_NAME
            ),
        };
        Ok(Self { channel_index })
    }

    pub fn channel(&self) -> Sig<MidiChannelUdp> {
        server::channel(self.channel_index)
    }
}

pub struct ByTitle<T: Clone>(Mutex<HashMap<String, T>>);

impl<T: Clone> Default for ByTitle<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: Clone> ByTitle<T> {
    pub fn get_or_insert<F: FnOnce() -> T>(
        &self,
        title: impl AsRef<str>,
        f: F,
    ) -> T {
        let title = title.as_ref().to_string();
        let mut table = self.0.lock().unwrap();
        if let Some(t) = table.get(&title) {
            t.clone()
        } else {
            let t = f();
            table.insert(title, t.clone());
            t
        }
    }
}

pub type MidiController01Udp = MidiController01<MidiChannelUdp>;
