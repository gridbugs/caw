use caw_core::{Sig, SigT};
use caw_midi::MidiMessages;
use midly::{MidiMessage, live::LiveEvent, num::u4};
use std::{
    io,
    net::{ToSocketAddrs, UdpSocket},
};

const BUF_SIZE: usize = 256;

struct MidiEvent {
    channel: u4,
    message: MidiMessage,
}

pub struct MidiLiveUdp {
    socket: UdpSocket,
    buf: Vec<u8>,
}

impl MidiLiveUdp {
    pub fn new<A: ToSocketAddrs>(addrs: A) -> anyhow::Result<Self> {
        let socket = UdpSocket::bind(addrs)?;
        socket.set_nonblocking(true)?;
        let buf = vec![0; BUF_SIZE];
        Ok(Self { socket, buf })
    }

    fn recv_into_buf(&mut self) -> Result<bool, io::Error> {
        match self.socket.recv(&mut self.buf) {
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
            match LiveEvent::parse(&self.buf) {
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

    pub fn channel(
        mut self,
        channel: u8,
    ) -> Sig<impl SigT<Item = MidiMessages>> {
        Sig::from_buf_fn(move |ctx, buf: &mut Vec<MidiMessages>| {
            // This is called once per frame (not once per sample). This will add an imperceptible
            // amount of latency (unless the output buffer is too large!), but reduce cpu usage.
            buf.resize_with(ctx.num_samples, Default::default);
            let mut midi_messages = MidiMessages::empty();
            loop {
                match self.recv_midi_event() {
                    Err(e) => {
                        log::warn!("IO error reading from UDP socket: {e}");
                        break;
                    }
                    Ok(None) => break,
                    Ok(Some(MidiEvent {
                        channel: message_channel,
                        message,
                    })) => {
                        if message_channel == channel {
                            midi_messages.push(message);
                        }
                    }
                }
            }
            // Only the first sample of each frame is populated with midi messages.
            buf[0] = midi_messages;
        })
    }
}
