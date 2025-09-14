use caw_midi::MidiEvent;
use midly::live::LiveEvent;
pub use midly::{MidiMessage, num::u4};
use std::net::{Ipv4Addr, ToSocketAddrs, UdpSocket};

pub struct MidiUdpClient {
    socket: UdpSocket,
}

impl MidiUdpClient {
    pub fn new<A: ToSocketAddrs>(addrs: A) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
        socket.connect(addrs)?;
        Ok(Self { socket })
    }

    pub fn send(
        &self,
        MidiEvent { channel, message }: MidiEvent,
    ) -> anyhow::Result<()> {
        let event = LiveEvent::Midi { channel, message };
        let mut buf = Vec::new();
        match event.write_std(&mut buf) {
            Ok(()) => (),
            Err(e) => anyhow::bail!("{e}"),
        }
        self.socket.send(&buf)?;
        Ok(())
    }
}
