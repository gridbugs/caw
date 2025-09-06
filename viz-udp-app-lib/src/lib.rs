//! A wrapper for the `caw_viz_udp_app` command. That command starts a udp client and opens a
//! window. The client listens for messages from a server representing vizualizations and renders
//! them in the window. This library provides the server implementation. Synthesizer code is
//! expected to use this library to create a udp server and spawn a client which connects to it,
//! and then manipulate the server to send visualization commands to the client. This library also
//! contains helper code for writing client applications.

pub mod blink;
mod common;
pub mod oscilloscope;
