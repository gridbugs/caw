use caw_midi::MidiEvent;
use caw_midi_udp_client::{MidiMessage, MidiUdpClient};
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    server: String,
    #[arg(long, default_value_t = 0)]
    channel: u8,
    #[arg(short, long, default_value_t = 0)]
    controller: u8,
    #[arg(short, long)]
    value: u8,
}

fn main() {
    let args = Args::parse();
    let client = MidiUdpClient::new(args.server).unwrap();
    let midi_event = MidiEvent {
        channel: args.channel.into(),
        message: MidiMessage::Controller {
            controller: args.controller.into(),
            value: args.value.into(),
        },
    };
    client.send(midi_event).unwrap();
}
