//! A wrapper for the `caw_viz_udp_app` command. That command starts a udp client and opens a
//! window. The client listens for messages from a server representing vizualizations and renders
//! them in the window. This library provides the server implementation. Synthesizer code is
//! expected to use this library to create a udp server and spawn a client which connects to it,
//! and then manipulate the server to send visualization commands to the client. This library also
//! contains helper code for writing client applications.

pub mod blink;
mod common;
pub mod oscilloscope;
use std::{collections::HashMap, sync::Mutex};

pub use common::SendStatus;

use caw_core::{Channel, Sig, SigT};
use lazy_static::lazy_static;

struct StereoOscilloscopeState {
    server: oscilloscope::Server,
    last_batch_index: u64,
    buf: Vec<f32>,
}

lazy_static! {
    static ref STEREO_OSCILLOSCOPE_STATES_BY_TITLE: Mutex<HashMap<String, StereoOscilloscopeState>> =
        Mutex::new(HashMap::new());
}

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

    /// Calling this function twice with the same `title`, passing `Channel::Left` to one
    /// invocation and `Channel::Right` to the other, will open a single window which receives a
    /// stream of interleaved samples from both signals.
    fn viz_oscilloscope_stereo(
        self,
        title: impl AsRef<str>,
        channel: Channel,
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

    fn viz_oscilloscope_stereo(
        self,
        title: impl AsRef<str>,
        channel: Channel,
        config: oscilloscope::Config,
    ) -> Sig<impl SigT<Item = f32>> {
        let title = title.as_ref().to_string();
        let make_server = {
            let title = title.clone();
            move || {
                oscilloscope::Server::new(title.as_str(), config.clone())
                    .unwrap()
            }
        };
        {
            // The first time this is called, set up the server and store it in a global.
            let mut states =
                STEREO_OSCILLOSCOPE_STATES_BY_TITLE.lock().unwrap();
            states.entry(title.clone()).or_insert_with(|| {
                StereoOscilloscopeState {
                    server: make_server(),
                    last_batch_index: 0,
                    buf: Vec::new(),
                }
            });
        }
        Sig(self).with_buf_ctx(move |buf, ctx| {
            let mut states =
                STEREO_OSCILLOSCOPE_STATES_BY_TITLE.lock().unwrap();
            let this_state = states.get_mut(&title).unwrap();
            if this_state.last_batch_index == ctx.batch_index {
                // The other channel must have already stored its samples in the buffer, so add the
                // current channel's samples and then send the buffer.
                let mut i = match channel {
                    Channel::Left => 0,
                    Channel::Right => 1,
                };
                for &sample in buf {
                    this_state.buf[i] = sample;
                    i += 2;
                }
                match this_state.server.send_samples(&this_state.buf) {
                    Ok(SendStatus::Success) => (),
                    Ok(SendStatus::Disconnected) => {
                        // re-open the visualization if it was closed
                        this_state.server = make_server();
                    }
                    Err(e) => eprintln!("{}", e),
                }
            } else {
                // The current channel is the first of the two channels to be computed this frame.
                // Add the current channel's samples to the buffer but don't send the buffer. The
                // other channel will send the buffer once it's added its samples.
                this_state.last_batch_index = ctx.batch_index;
                this_state.buf.clear();
                for &sample in buf {
                    // Add the sample twice. One of these samples will be for the left channel and
                    // one will be for the right channel. We don't care which one we are at this
                    // point. The other channel will overwrite its entries in the buffer before
                    // sending.
                    this_state.buf.push(sample);
                    this_state.buf.push(sample);
                }
            }
        })
    }
}
