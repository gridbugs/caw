use caw_core_next::{FrameSig, FrameSigT};
use caw_midi::MidiMessages;
use midly::live::LiveEvent;
use nix::sys::termios::BaudRate;
use std::{os::fd::OwnedFd, path::Path};

pub struct MidiLiveSerial {
    owned_fd: OwnedFd,
}

impl MidiLiveSerial {
    fn convert_baud_rate(baud_rate: u32) -> anyhow::Result<BaudRate> {
        Ok(match baud_rate {
            0 => BaudRate::B0,
            50 => BaudRate::B50,
            75 => BaudRate::B75,
            110 => BaudRate::B110,
            134 => BaudRate::B134,
            150 => BaudRate::B150,
            200 => BaudRate::B200,
            300 => BaudRate::B300,
            600 => BaudRate::B600,
            1200 => BaudRate::B1200,
            1800 => BaudRate::B1800,
            2400 => BaudRate::B2400,
            4800 => BaudRate::B4800,
            9600 => BaudRate::B9600,
            19200 => BaudRate::B19200,
            38400 => BaudRate::B38400,
            57600 => BaudRate::B57600,
            115200 => BaudRate::B115200,
            230400 => BaudRate::B230400,
            other => anyhow::bail!("Unsupported baud rate: {}", other),
        })
    }

    pub fn new(
        tty_path: impl AsRef<Path>,
        baud_rate: u32,
    ) -> anyhow::Result<Self> {
        use nix::{
            fcntl::{self, OFlag},
            sys::{
                stat::Mode,
                termios::{self, LocalFlags, SetArg},
            },
        };
        let fd = fcntl::open(
            tty_path.as_ref(),
            OFlag::O_RDONLY | OFlag::O_NONBLOCK,
            Mode::empty(),
        )?;
        let owned_fd = unsafe {
            use std::os::fd::FromRawFd;
            // fd is a valid file descriptor and the only cleanup it needs is close
            OwnedFd::from_raw_fd(fd)
        };
        let mut termios = termios::tcgetattr(&owned_fd)?;
        termios.local_flags &= !(LocalFlags::ECHO | LocalFlags::ICANON);
        let baud_rate = Self::convert_baud_rate(baud_rate)?;
        termios::cfsetspeed(&mut termios, baud_rate)?;
        termios::tcsetattr(&owned_fd, SetArg::TCSANOW, &termios)?;
        Ok(MidiLiveSerial { owned_fd })
    }

    fn read_byte(&self) -> anyhow::Result<Option<u8>> {
        use nix::{errno::Errno, unistd};
        use std::os::fd::AsRawFd;
        let mut buf = [0];
        let nbytes = match unistd::read(self.owned_fd.as_raw_fd(), &mut buf) {
            Ok(nbytes) => nbytes,
            Err(Errno::EAGAIN) => 0,
            Err(e) => return Err(e.into()),
        };
        if nbytes == 0 {
            Ok(None)
        } else {
            let [byte] = buf;
            Ok(Some(byte))
        }
    }

    fn read_all_available(&self, buf: &mut Vec<u8>) -> anyhow::Result<()> {
        while let Some(byte) = self.read_byte()? {
            buf.push(byte);
        }
        Ok(())
    }

    /// This consumes `self` as only a single channel may be subscribed to at a time. This
    /// constraint is purely for simplicity and can be lifted eventually if necessary.
    pub fn channel(
        self,
        channel: u8,
    ) -> FrameSig<impl FrameSigT<Item = MidiMessages>> {
        let mut raw_buf = Vec::new();
        let mut message_buf = Vec::new();
        FrameSig::from_fn(move |_ctx| {
            let mut midi_messages = MidiMessages::empty();
            raw_buf.clear();
            if let Ok(()) = self.read_all_available(&mut raw_buf) {
                let mut drain = raw_buf.drain(..).peekable();
                // skip bytes until we get to the first midi status byte
                while let Some(&first) = drain.peek() {
                    if first > 127 {
                        // midi status byte
                        break;
                    }
                    drain.next();
                }
                message_buf.clear();
                while let Some(byte) = drain.next() {
                    message_buf.push(byte);
                    // Keep reading bytes until we reach the end of the buffer or the next byte
                    // would be the start of a new message.
                    if let Some(&next_byte) = drain.peek() {
                        if next_byte <= 127 {
                            continue;
                        }
                    }
                    // Hopefully at this point `message_buf` contains a valid midi message. Attempt
                    // to parse it.
                    if let Ok(LiveEvent::Midi {
                        channel: message_channel,
                        message,
                    }) = LiveEvent::parse(&message_buf)
                    {
                        // Discard messages that aren't meant for the requested channel. Eventually
                        // we may want to support subscribing to multiple channels at once but for
                        // simplicity we'll assume for now that only one channel can be subscribed.
                        if message_channel == channel {
                            midi_messages.push(message);
                        }
                    }
                }
            }
            midi_messages
        })
    }
}
