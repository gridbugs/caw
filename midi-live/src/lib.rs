use midir::{MidiInput, MidiInputConnection, MidiInputPort};
use midly::{live::LiveEvent, Format, Smf};
use midly::{
    num::{u14, u4, u7},
    MidiMessage,
};
use std::sync::mpsc;

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
}

struct MidiLiveConnection {
    #[allow(unused)]
    midi_input_connection: MidiInputConnection<()>,
    midi_event_receiver: mpsc::Receiver<MidiEvent>,
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
                    if let Ok(event) = LiveEvent::parse(message) {
                        if let LiveEvent::Midi { channel, message } = event {
                            let midi_event = MidiEvent { channel, message };
                            if midi_event_sender.send(midi_event).is_err() {
                                log::error!("failed to send message from live midi thread");
                            }
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
