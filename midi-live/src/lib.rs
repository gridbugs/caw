use caw_core::{FrameSig, FrameSigT};
use caw_midi::MidiMessages;
use midir::{MidiInput, MidiInputConnection, MidiInputPort};
use midly::live::LiveEvent;
use midly::{num::u4, MidiMessage};
use std::{cell::RefCell, mem, rc::Rc, sync::mpsc};

struct MidiEvent {
    pub channel: u4,
    pub message: MidiMessage,
}

pub struct MidiLive {
    midi_input: MidiInput,
    midi_input_ports: Vec<MidiInputPort>,
}

impl MidiLive {
    pub fn new() -> anyhow::Result<Self> {
        let midi_input = MidiInput::new("caw")?;
        let midi_input_ports = midi_input.ports();
        Ok(Self {
            midi_input,
            midi_input_ports,
        })
    }

    pub fn enumerate_port_names(
        &self,
    ) -> impl Iterator<Item = (usize, String)> + '_ {
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

    pub fn connect(
        self,
        port_index: usize,
    ) -> anyhow::Result<MidiLiveConnection> {
        let port = &self.midi_input_ports[port_index];
        MidiLiveConnection::new(self.midi_input, port)
    }
}

const NUM_CHANNELS: usize = 16;

#[derive(Default)]
struct MessageBuffer {
    messages: MidiMessages,
    subscribed: bool,
}

struct MessageBuffers {
    buffers: [MessageBuffer; NUM_CHANNELS],
    midi_event_receiver: mpsc::Receiver<MidiEvent>,
}

impl MessageBuffers {
    pub fn update(&mut self) {
        for MidiEvent { channel, message } in
            self.midi_event_receiver.try_iter()
        {
            let buffer = &mut self.buffers[channel.as_int() as usize];
            if buffer.subscribed {
                buffer.messages.push(message);
            }
        }
    }
}

pub struct MidiLiveConnection {
    #[allow(unused)]
    midi_input_connection: MidiInputConnection<()>,
    message_buffers: Rc<RefCell<MessageBuffers>>,
}

impl MidiLiveConnection {
    fn new(
        midi_input: MidiInput,
        port: &MidiInputPort,
    ) -> anyhow::Result<Self> {
        let port_name = format!("caw {}", midi_input.port_name(port)?);
        let (midi_event_sender, midi_event_receiver) =
            mpsc::channel::<MidiEvent>();
        let midi_input_connection = midi_input
            .connect(
                port,
                port_name.as_str(),
                move |_timestamp_us, message, &mut ()| {
                    if let Ok(LiveEvent::Midi { channel, message }) =
                        LiveEvent::parse(message)
                    {
                        let midi_event = MidiEvent { channel, message };
                        if midi_event_sender.send(midi_event).is_err() {
                            log::error!(
                                "failed to send message from live midi thread"
                            );
                        }
                    }
                },
                (),
            )
            .map_err(|_| anyhow::anyhow!("Failed to connect to midi port"))?;
        Ok(Self {
            midi_input_connection,
            message_buffers: Rc::new(RefCell::new(MessageBuffers {
                buffers: Default::default(),
                midi_event_receiver,
            })),
        })
    }

    pub fn channel(
        &self,
        channel: u8,
    ) -> FrameSig<impl FrameSigT<Item = MidiMessages>> {
        {
            let mut message_buffers = self.message_buffers.borrow_mut();
            let buffer = &mut message_buffers.buffers[channel as usize];
            if buffer.subscribed {
                panic!("Midi channel {} subscribed to multiple times. Each midi channel may be subscribed to only once.", channel);
            }
            buffer.subscribed = true;
        }
        let message_buffers = Rc::clone(&self.message_buffers);
        FrameSig::from_fn(move |_ctx| {
            let mut message_buffers = message_buffers.borrow_mut();
            message_buffers.update();
            mem::take(&mut message_buffers.buffers[channel as usize].messages)
        })
    }
}
