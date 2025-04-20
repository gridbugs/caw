use caw_midi_udp_client::*;
use caw_widgets::*;
use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    server: String,
    #[arg(long, default_value_t = 0)]
    channel: u8,
    #[arg(short, long, default_value_t = 0)]
    controller: u8,
}

fn main() {
    let args = Args::parse();
    let client = MidiUdpClient::new(args.server).unwrap();
    let mut knob = Knob::new("foo bar", 0.5, 0.2).unwrap();
    loop {
        knob.tick().unwrap();
        client
            .send(MidiEvent {
                channel: args.channel.into(),
                message: MidiMessage::Controller {
                    controller: args.controller.into(),
                    value: knob.value_midi(),
                },
            })
            .unwrap();
    }
}
