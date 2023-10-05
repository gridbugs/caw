use crate::signal::SignalCtx;
pub use currawong_core::midi::*;
use midir::{MidiInput, MidiInputConnection, MidiInputPort};
use midly::{live::LiveEvent, Format, Smf};
use nix::sys::termios::BaudRate;
use std::{fs, os::fd::OwnedFd, path::Path, sync::mpsc};

pub struct MidiFile {
    smf: Smf<'static>,
}

impl MidiFile {
    pub fn read(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let raw = fs::read(path)?;
        let smf = Smf::parse(&raw)?.make_static();
        Ok(Self { smf })
    }

    pub fn format(&self) -> Format {
        self.smf.header.format
    }

    pub fn num_tracks(&self) -> usize {
        self.smf.tracks.len()
    }

    pub fn track_player(
        &self,
        track_index: usize,
        channel: u8,
        polyphony: usize,
        default_s_per_beat: f64,
    ) -> anyhow::Result<MidiPlayer> {
        if let Some(events) = self.smf.tracks.get(track_index) {
            let event_source =
                TrackEventSource::new(events, self.smf.header.timing, default_s_per_beat);
            Ok(MidiPlayer::new(channel.into(), polyphony, event_source))
        } else {
            anyhow::bail!(
                "Track index {} is out of range (there are {} tracks)",
                track_index,
                self.num_tracks()
            )
        }
    }
}

pub struct MidiLive {
    midi_input: MidiInput,
    midi_input_ports: Vec<MidiInputPort>,
}

impl MidiLive {
    pub fn new() -> anyhow::Result<Self> {
        let midi_input = MidiInput::new("currawong")?;
        let midi_input_ports = midi_input.ports();
        Ok(Self {
            midi_input,
            midi_input_ports,
        })
    }

    pub fn enumerate_port_names<'a>(&'a self) -> impl Iterator<Item = (usize, String)> + 'a {
        self.midi_input_ports
            .iter()
            .enumerate()
            .filter_map(|(i, port)| {
                if let Ok(name) = self.midi_input.port_name(port) {
                    Some((i, name))
                } else {
                    None
                }
            })
    }

    pub fn into_player(
        self,
        port_index: usize,
        channel: u8,
        polyphony: usize,
    ) -> anyhow::Result<MidiPlayer> {
        let event_source =
            MidirMidiInputEventSource::new(self.midi_input, &self.midi_input_ports[port_index])?;
        Ok(MidiPlayer::new(channel.into(), polyphony, event_source))
    }

    pub fn into_player_monophonic(
        self,
        port_index: usize,
        channel: u8,
    ) -> anyhow::Result<MidiPlayerMonophonic> {
        let event_source =
            MidirMidiInputEventSource::new(self.midi_input, &self.midi_input_ports[port_index])?;
        Ok(MidiPlayerMonophonic::new(channel.into(), event_source))
    }
}

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

    pub fn new(tty_path: impl AsRef<Path>, baud_rate: u32) -> anyhow::Result<Self> {
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

    pub fn into_player(self, channel: u8, polyphony: usize) -> MidiPlayer {
        let event_source = MidiLiveSerialEventSource::new(self);
        MidiPlayer::new(channel.into(), polyphony, event_source)
    }
}

struct MidirMidiInputEventSource {
    #[allow(unused)]
    midi_input_connection: MidiInputConnection<()>,
    midi_event_receiver: mpsc::Receiver<MidiEvent>,
}

impl MidirMidiInputEventSource {
    fn new(midi_input: MidiInput, port: &MidiInputPort) -> anyhow::Result<Self> {
        let port_name = format!("currawong {}", midi_input.port_name(port)?);
        let (midi_event_sender, midi_event_receiver) = mpsc::channel::<MidiEvent>();
        let midi_input_connection = midi_input
            .connect(
                port,
                port_name.as_str(),
                move |_timestamp_us, message, &mut ()| {
                    if let Ok(event) = LiveEvent::parse(message) {
                        match event {
                            LiveEvent::Midi { channel, message } => {
                                let midi_event = MidiEvent { channel, message };
                                if let Err(_) = midi_event_sender.send(midi_event) {
                                    log::error!("failed to send message from live midi thread");
                                }
                            }
                            _ => (),
                        }
                    }
                },
                (),
            )
            .map_err(|_| anyhow::anyhow!("Failed to connect to midi port"))?;
        Ok(Self {
            midi_input_connection,
            midi_event_receiver,
        })
    }
}

impl MidiEventSource for MidirMidiInputEventSource {
    fn for_each_new_event<F>(&mut self, _ctx: &SignalCtx, f: F)
    where
        F: FnMut(MidiEvent),
    {
        self.midi_event_receiver.try_iter().for_each(f)
    }
}

struct MidiLiveSerialEventSource {
    midi_live_serial: MidiLiveSerial,
    buf: Vec<u8>,
    message_buf: Vec<u8>,
}

impl MidiLiveSerialEventSource {
    fn new(midi_live_serial: MidiLiveSerial) -> Self {
        Self {
            midi_live_serial,
            buf: Vec::new(),
            message_buf: Vec::new(),
        }
    }
}

impl MidiEventSource for MidiLiveSerialEventSource {
    fn for_each_new_event<F>(&mut self, _ctx: &SignalCtx, mut f: F)
    where
        F: FnMut(MidiEvent),
    {
        self.buf.clear();
        if let Ok(()) = self.midi_live_serial.read_all_available(&mut self.buf) {
            if self.buf.is_empty() {
                return;
            }
            let mut drain = self.buf.drain(..).peekable();
            // skip bytes until we get to the first midi status byte
            while let Some(&first) = drain.peek() {
                if first > 127 {
                    // midi status byte
                    break;
                }
                drain.next();
            }
            self.message_buf.clear();
            while let Some(byte) = drain.next() {
                self.message_buf.push(byte);
                if let Some(&next_byte) = drain.peek() {
                    if next_byte <= 127 {
                        continue;
                    }
                }
                if let Ok(event) = LiveEvent::parse(&self.message_buf) {
                    match event {
                        LiveEvent::Midi { channel, message } => f(MidiEvent { channel, message }),
                        _ => (),
                    }
                }
                self.message_buf.clear();
            }
        }
    }
}
