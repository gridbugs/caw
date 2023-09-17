use crate::{
    music,
    signal::{Freq, Gate, Sf64, Sfreq, Signal, SignalCtx},
};
use midir::{MidiInput, MidiInputConnection, MidiInputPort};
use midly::{
    live::LiveEvent, Format, MetaMessage, PitchBend, Smf, Timing, TrackEvent, TrackEventKind,
};
pub use midly::{
    num::{u14, u4, u7},
    MidiMessage,
};
use nix::sys::termios::BaudRate;
use std::{cell::RefCell, fs, os::fd::OwnedFd, path::Path, rc::Rc, sync::mpsc};

#[derive(Debug, Clone, Copy)]
pub struct MidiEvent {
    pub channel: u4,
    pub message: MidiMessage,
}

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
    ) -> anyhow::Result<MidiPlayer> {
        if let Some(events) = self.smf.tracks.get(track_index) {
            let event_source = MidlyTrackEventSource::new(events, self.smf.header.timing);
            Ok(MidiPlayerRaw::new(channel.into(), polyphony, event_source).to_player())
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
        let midi_input = MidiInput::new("ibis")?;
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
        let player = MidiPlayerRaw::new(channel.into(), polyphony, event_source).to_player();
        Ok(player)
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
            460800 => BaudRate::B460800,
            500000 => BaudRate::B500000,
            576000 => BaudRate::B576000,
            921600 => BaudRate::B921600,
            1000000 => BaudRate::B1000000,
            1152000 => BaudRate::B1152000,
            1500000 => BaudRate::B1500000,
            2000000 => BaudRate::B2000000,
            2500000 => BaudRate::B2500000,
            3000000 => BaudRate::B3000000,
            3500000 => BaudRate::B3500000,
            4000000 => BaudRate::B4000000,
            other => other.try_into()?,
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
        MidiPlayerRaw::new(channel.into(), polyphony, event_source).to_player()
    }
}

pub struct MidiVoiceRaw {
    pub note_index: Signal<u7>,
    pub velocity: Signal<u7>,
    pub gate: Gate,
}

pub struct MidiVoice {
    pub note_freq: Sfreq,
    pub velocity_01: Sf64,
    pub gate: Gate,
}

fn signal_u7_to_01(s: &Signal<u7>) -> Sf64 {
    s.map(|x| x.as_int() as f64 / 127.0)
}

impl MidiVoiceRaw {
    pub fn to_voice(&self) -> MidiVoice {
        MidiVoice {
            note_freq: self
                .note_index
                .map(|i| Freq::from_hz(music::freq_hz_of_midi_index(i.as_int()))),
            velocity_01: signal_u7_to_01(&self.velocity),
            gate: self.gate.clone(),
        }
    }
}

pub struct MidiControllerTableRaw {
    values: Vec<Signal<u7>>,
}

pub struct MidiControllerTable {
    values_01: Vec<Sf64>,
}

impl MidiControllerTableRaw {
    pub fn get(&self, i: u7) -> Signal<u7> {
        self.values[i.as_int() as usize].clone()
    }

    pub fn to_controller_table(&self) -> MidiControllerTable {
        MidiControllerTable {
            values_01: self.values.iter().map(signal_u7_to_01).collect(),
        }
    }
}

impl MidiControllerTable {
    pub fn get_01(&self, i: u8) -> Sf64 {
        self.values_01[i as usize].clone()
    }

    pub fn modulation(&self) -> Sf64 {
        self.get_01(1)
    }
}

pub struct MidiPlayerRaw {
    pub voices: Vec<MidiVoiceRaw>,
    pub controllers: MidiControllerTableRaw,
    pub pitch_bend: Signal<u14>,
}

pub struct MidiPlayer {
    pub voices: Vec<MidiVoice>,
    pub controllers: MidiControllerTable,
    pub pitch_bend_1: Sf64,
    pub pitch_bend_multiplier: Sf64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GateState {
    Pressed,
    Released,
    ReleasedThenPressed,
}

struct MidiPlayerRawStateVoice {
    note_index: u7,
    velocity: u7,
    gate: GateState,
    last_change_tick: u64,
}

struct MidiPlayerRawState {
    // a value of `None` indicates that the voice is free to be used for new notes without
    // interrupting a currently-playing sound
    voices: Vec<Option<MidiPlayerRawStateVoice>>,
    controllers: Vec<u7>,
    pitch_bend: u14,
    current_tick: u64,
}

impl MidiPlayerRawState {
    fn allocate_voice(&self) -> usize {
        for (i, voice) in self.voices.iter().enumerate() {
            if voice.is_none() {
                return i;
            }
        }
        // all voices are in use - try to allocate the the voice whose gate was opened first (of
        // voices whose gates are currently open)
        {
            let mut latest_tick = self.current_tick;
            let mut best_index = None;
            for (i, voice) in self.voices.iter().enumerate() {
                let voice = voice.as_ref().unwrap();
                if voice.gate == GateState::Released && voice.last_change_tick < latest_tick {
                    latest_tick = voice.last_change_tick;
                    best_index = Some(i);
                }
            }
            if let Some(i) = best_index {
                return i;
            }
        }
        // all gates are currently closed - allocate the most recently pressed voice
        let mut best_index = 0;
        let mut oldest_tick = 0;
        for (i, voice) in self.voices.iter().enumerate() {
            let voice = voice.as_ref().unwrap();
            if voice.last_change_tick > oldest_tick {
                oldest_tick = voice.last_change_tick;
                best_index = i;
            }
        }
        best_index
    }

    fn note_on(&mut self, note_index: u7, velocity: u7) {
        if velocity == 0 {
            self.note_off(note_index);
            return;
        }

        let mut updated = false;
        for voice in self.voices.iter_mut() {
            if let Some(voice) = voice.as_mut() {
                if voice.note_index == note_index {
                    voice.velocity = velocity;
                    voice.gate = if voice.last_change_tick == self.current_tick
                        && voice.gate == GateState::Released
                    {
                        GateState::ReleasedThenPressed
                    } else {
                        GateState::Pressed
                    };
                    voice.last_change_tick = self.current_tick;
                    updated = true;
                }
            }
        }
        if updated {
            return;
        }
        let voice_index = self.allocate_voice();
        let voice = MidiPlayerRawStateVoice {
            note_index,
            velocity,
            gate: GateState::ReleasedThenPressed,
            last_change_tick: self.current_tick,
        };
        self.voices[voice_index] = Some(voice);
    }

    fn note_off(&mut self, note_index: u7) {
        for voice in self.voices.iter_mut() {
            if let Some(voice) = voice.as_mut() {
                if voice.note_index == note_index {
                    voice.gate = GateState::Released;
                    voice.last_change_tick = self.current_tick;
                }
            }
        }
    }

    fn progress_gates(&mut self) {
        for voice in self.voices.iter_mut() {
            if let Some(voice) = voice.as_mut() {
                if voice.gate == GateState::ReleasedThenPressed {
                    voice.gate = GateState::Pressed;
                }
            }
        }
    }
}

trait MidiEventSource {
    fn for_each_new_event<F>(&mut self, ctx: &SignalCtx, f: F)
    where
        F: FnMut(MidiEvent);
}

struct MidlyTrackEventSource {
    track: Vec<TrackEvent<'static>>,
    timing: Timing,
    tick_duration_s: f64,
    next_index: usize,
    next_tick: u32,
    current_time_s: f64,
    next_tick_time_s: f64,
}

const DEFAULT_US_PER_BEAT: u32 = 500_000;

impl MidlyTrackEventSource {
    fn new(track: &[TrackEvent<'static>], timing: Timing) -> Self {
        let track = track.to_vec();
        let tick_duration_s = match timing {
            Timing::Metrical(ticks_per_beat) => {
                DEFAULT_US_PER_BEAT as f64 / (ticks_per_beat.as_int() as f64 * 1_000_000.0)
            }
            Timing::Timecode(frames_per_second, ticks_per_frame) => {
                1.0 / (frames_per_second.as_f32() as f64 * ticks_per_frame as f64)
            }
        };
        Self {
            track,
            timing,
            tick_duration_s,
            next_index: 0,
            next_tick: 0,
            current_time_s: 0.0,
            next_tick_time_s: 0.0,
        }
    }
}

impl MidiEventSource for MidlyTrackEventSource {
    fn for_each_new_event<F>(&mut self, ctx: &SignalCtx, mut f: F)
    where
        F: FnMut(MidiEvent),
    {
        let time_since_prev_tick_s = 1.0 / ctx.sample_rate_hz;
        self.current_time_s += time_since_prev_tick_s;
        while self.next_tick_time_s < self.current_time_s {
            self.next_tick_time_s += self.tick_duration_s;
            while let Some(event) = self.track.get(self.next_index) {
                let tick = self.next_tick;
                if event.delta == tick {
                    self.next_tick = 0;
                    self.next_index += 1;
                    match event.kind {
                        TrackEventKind::Midi { channel, message } => {
                            f(MidiEvent { channel, message })
                        }
                        TrackEventKind::Meta(MetaMessage::Tempo(us_per_beat)) => {
                            if let Timing::Metrical(ticks_per_beat) = self.timing {
                                self.tick_duration_s = us_per_beat.as_int() as f64
                                    / (ticks_per_beat.as_int() as f64 * 1_000_000.0);
                            }
                        }
                        _ => (),
                    }
                } else {
                    break;
                }
            }
            self.next_tick += 1;
        }
    }
}

struct MidirMidiInputEventSource {
    #[allow(unused)]
    midi_input_connection: MidiInputConnection<()>,
    midi_event_receiver: mpsc::Receiver<MidiEvent>,
}

impl MidirMidiInputEventSource {
    fn new(midi_input: MidiInput, port: &MidiInputPort) -> anyhow::Result<Self> {
        let port_name = format!("ibis {}", midi_input.port_name(port)?);
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

impl MidiPlayerRaw {
    fn new(
        channel: u4,
        polyphony: usize,
        mut event_source: impl MidiEventSource + 'static,
    ) -> MidiPlayerRaw {
        let state = Rc::new(RefCell::new(MidiPlayerRawState {
            voices: (0..polyphony).map(|_| None).collect(),
            controllers: (0..128).map(|_| u7::new(0)).collect(),
            pitch_bend: u14::new(0x2000),
            current_tick: 0,
        }));
        let effectful_signal = Signal::from_fn({
            let state = Rc::clone(&state);
            move |ctx| {
                let mut state = state.borrow_mut();
                state.progress_gates();
                event_source.for_each_new_event(ctx, |event| {
                    if event.channel == channel {
                        use MidiMessage::*;
                        match event.message {
                            NoteOn { key, vel } => state.note_on(key, vel),
                            NoteOff { key, vel: _ } => state.note_off(key),
                            PitchBend {
                                bend: PitchBend(pitch_bend),
                            } => state.pitch_bend = pitch_bend,
                            Controller { controller, value } => {
                                state.controllers[controller.as_int() as usize] = value;
                            }
                            _ => (),
                        }
                    }
                });
                state.current_tick += 1;
            }
        });
        let voices = (0..polyphony)
            .map(|i| {
                let note_index = Signal::from_fn({
                    let state = Rc::clone(&state);
                    let effectful_signal = effectful_signal.clone();
                    move |ctx| {
                        effectful_signal.sample(ctx);
                        state.borrow().voices[i]
                            .as_ref()
                            .map(|v| v.note_index)
                            .unwrap_or(u7::new(0))
                    }
                });
                let velocity = Signal::from_fn({
                    let state = Rc::clone(&state);
                    let effectful_signal = effectful_signal.clone();
                    move |ctx| {
                        effectful_signal.sample(ctx);
                        state.borrow().voices[i]
                            .as_ref()
                            .map(|v| v.velocity)
                            .unwrap_or(u7::new(0))
                    }
                });
                let gate = Gate::from_fn({
                    let state = Rc::clone(&state);
                    let effectful_signal = effectful_signal.clone();
                    move |ctx| {
                        effectful_signal.sample(ctx);
                        let gate_state = state.borrow().voices[i]
                            .as_ref()
                            .map(|v| v.gate)
                            .unwrap_or(GateState::Released);
                        gate_state == GateState::Pressed
                    }
                });
                MidiVoiceRaw {
                    note_index,
                    velocity,
                    gate,
                }
            })
            .collect::<Vec<_>>();
        let controllers = MidiControllerTableRaw {
            values: (0..128)
                .map(|i| {
                    Signal::from_fn({
                        let state = Rc::clone(&state);
                        let effectful_signal = effectful_signal.clone();
                        move |ctx| {
                            effectful_signal.sample(ctx);
                            state.borrow().controllers[i]
                        }
                    })
                })
                .collect(),
        };
        let pitch_bend = Signal::from_fn({
            let state = Rc::clone(&state);
            let effectful_signal = effectful_signal.clone();
            move |ctx| {
                effectful_signal.sample(ctx);
                state.borrow().pitch_bend
            }
        });
        Self {
            voices,
            controllers,
            pitch_bend,
        }
    }

    pub fn to_player(&self) -> MidiPlayer {
        let pitch_bend_1 = self
            .pitch_bend
            .map(|x| ((x.as_int() as i32 - 0x2000) as f64 * 2.0) / 0x3FFF as f64);
        let pitch_bend_multiplier = pitch_bend_1.map({
            let max_ratio = music::semitone_ratio(2.0);
            move |x| max_ratio.powf(x)
        });
        MidiPlayer {
            voices: self.voices.iter().map(|v| v.to_voice()).collect(),
            controllers: self.controllers.to_controller_table(),
            pitch_bend_1,
            pitch_bend_multiplier,
        }
    }
}
