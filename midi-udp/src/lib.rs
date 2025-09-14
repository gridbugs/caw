use caw_core::{Buf, SigCtx, SigT};
use caw_midi::{MidiEvent, MidiEvents};
use midly::live::LiveEvent;
use std::{
    io,
    net::{Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket},
};

const BUF_SIZE: usize = 256;

pub struct MidiLiveUdp {
    socket: UdpSocket,
    buf_raw: Vec<u8>,
    buf: Vec<MidiEvents>,
}

impl MidiLiveUdp {
    pub fn new<A: ToSocketAddrs>(addrs: A) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind(addrs)?;
        socket.set_nonblocking(true)?;
        let buf_raw = vec![0; BUF_SIZE];
        Ok(Self {
            socket,
            buf_raw,
            buf: Vec::new(),
        })
    }

    pub fn new_unspecified() -> anyhow::Result<Self> {
        Self::new((Ipv4Addr::UNSPECIFIED, 0))
    }

    pub fn local_socket_address(&self) -> anyhow::Result<SocketAddr> {
        Ok(self.socket.local_addr()?)
    }

    fn recv_into_buf(&mut self) -> Result<bool, io::Error> {
        match self.socket.recv(&mut self.buf_raw) {
            Ok(size) => {
                if size >= BUF_SIZE {
                    log::warn!("UDP message too long for buffer!");
                    Ok(false)
                } else {
                    Ok(true)
                }
            }
            Err(error) => match error.kind() {
                io::ErrorKind::WouldBlock => {
                    // There is currently no datagram available.
                    Ok(false)
                }
                _ => Err(error),
            },
        }
    }

    fn recv_midi_event(&mut self) -> Result<Option<MidiEvent>, io::Error> {
        if self.recv_into_buf()? {
            match LiveEvent::parse(&self.buf_raw) {
                Ok(LiveEvent::Midi { channel, message }) => {
                    Ok(Some(MidiEvent { channel, message }))
                }
                Ok(_) => Ok(None),
                Err(e) => {
                    log::warn!("Failed to parse midi event: {e}");
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }
}

impl SigT for MidiLiveUdp {
    type Item = MidiEvents;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        // This is called once per frame (not once per sample). This will add an imperceptible
        // amount of latency (unless the output buffer is too large!), but reduce cpu usage.
        self.buf.resize_with(ctx.num_samples, Default::default);
        let mut midi_events = MidiEvents::empty();
        loop {
            match self.recv_midi_event() {
                Err(e) => {
                    log::warn!("IO error reading from UDP socket: {e}");
                    break;
                }
                Ok(None) => break,
                Ok(Some(midi_event)) => {
                    midi_events.push(midi_event);
                }
            }
        }
        // Only the first sample of each frame is populated with midi messages.
        self.buf[0] = midi_events;
        &self.buf
    }
}
