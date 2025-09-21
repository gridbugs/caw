use caw_core::{Sig, SigShared, sig_shared};
use caw_midi::{MidiChannel, MidiEventsT};
use caw_midi_udp::MidiLiveUdp;
use lazy_static::lazy_static;
use midly::num::u4;
use std::net::SocketAddr;

pub type MidiChannelUdp = MidiChannel<SigShared<MidiLiveUdp>>;

lazy_static! {
    // A stream of MIDI events received by the UDP/IP server. It owns the UDP/IP server.
    static ref SIG: Sig<SigShared<MidiLiveUdp>> =
        sig_shared(MidiLiveUdp::new_unspecified().unwrap());

}

/// Returns the IP address of the server that will receive MIDI events.
pub fn local_socket_address() -> SocketAddr {
    SIG.0.with_inner(|midi_live_udp| {
        midi_live_udp.local_socket_address().unwrap()
    })
}

pub fn channel(channel: u4) -> Sig<MidiChannelUdp> {
    SIG.clone().channel(channel.into())
}
