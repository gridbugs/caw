use crate::server::{self, MidiChannelUdp};
use caw_core::{Sig, SigShared, SigT, sig_shared};
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

pub struct ByTitle<T: SigT>(Mutex<HashMap<String, Sig<SigShared<T>>>>);

impl<T: SigT> Default for ByTitle<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: SigT> ByTitle<T> {
    pub fn get_or_insert<F: FnOnce() -> T>(
        &self,
        title: impl AsRef<str>,
        f: F,
    ) -> Sig<SigShared<T>> {
        let title = title.as_ref().to_string();
        let mut table = self.0.lock().unwrap();
        if let Some(t_shared) = table.get(&title) {
            t_shared.clone()
        } else {
            let t_shared = sig_shared(f());
            table.insert(title, t_shared.clone());
            t_shared
        }
    }
}
