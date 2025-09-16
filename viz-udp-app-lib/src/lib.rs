//! A wrapper for the `caw_viz_udp_app` command. That command starts a udp client and opens a
//! window. The client listens for messages from a server representing vizualizations and renders
//! them in the window. This library provides the server implementation. Synthesizer code is
//! expected to use this library to create a udp server and spawn a client which connects to it,
//! and then manipulate the server to send visualization commands to the client. This library also
//! contains helper code for writing client applications.

pub mod blink;
mod common;
pub mod oscilloscope;
pub use common::SendStatus;

use caw_core::{Sig, SigT};

pub trait VizSigBool {
    fn viz_blink(
        self,
        title: impl AsRef<str>,
        config: blink::Config,
    ) -> Sig<impl SigT<Item = bool>>;
}

impl<S> VizSigBool for Sig<S>
where
    S: SigT<Item = bool>,
{
    fn viz_blink(
        self,
        title: impl AsRef<str>,
        config: blink::Config,
    ) -> Sig<impl SigT<Item = bool>> {
        let make_viz =
            move || blink::Server::new(title.as_ref(), config.clone()).unwrap();
        let mut viz = make_viz();
        let sig = self.shared();
        let delay_s = 0.05;
        let env =
            caw_modules::adsr_linear_01(sig.clone().trig_to_gate(delay_s))
                .release_s(delay_s)
                .build()
                .with_buf(move |buf| {
                    match viz.send_samples(buf) {
                        Ok(SendStatus::Success) => (),
                        Ok(SendStatus::Disconnected) => {
                            // re-open the visualization if it was closed
                            viz = make_viz();
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                });
        Sig(sig).force(env)
    }
}

pub trait VizSigF32 {
    fn viz_blink(
        self,
        title: impl AsRef<str>,
        config: blink::Config,
    ) -> Sig<impl SigT<Item = f32>>;

    fn viz_oscilloscope(
        self,
        title: impl AsRef<str>,
        config: oscilloscope::Config,
    ) -> Sig<impl SigT<Item = f32>>;
}

impl<S> VizSigF32 for Sig<S>
where
    S: SigT<Item = f32>,
{
    fn viz_blink(
        self,
        title: impl AsRef<str>,
        config: blink::Config,
    ) -> Sig<impl SigT<Item = f32>> {
        let make_viz =
            move || blink::Server::new(title.as_ref(), config.clone()).unwrap();
        let mut viz = make_viz();
        Sig(self).with_buf(move |buf| {
            match viz.send_samples(buf) {
                Ok(SendStatus::Success) => (),
                Ok(SendStatus::Disconnected) => {
                    // re-open the visualization if it was closed
                    viz = make_viz();
                }
                Err(e) => eprintln!("{}", e),
            }
        })
    }

    fn viz_oscilloscope(
        self,
        title: impl AsRef<str>,
        config: oscilloscope::Config,
    ) -> Sig<impl SigT<Item = f32>> {
        let make_viz = move || {
            oscilloscope::Server::new(title.as_ref(), config.clone()).unwrap()
        };
        let mut viz = make_viz();
        Sig(self).with_buf(move |buf| {
            match viz.send_samples(buf) {
                Ok(SendStatus::Success) => (),
                Ok(SendStatus::Disconnected) => {
                    // re-open the visualization if it was closed
                    viz = make_viz();
                }
                Err(e) => eprintln!("{}", e),
            }
        })
    }
}
