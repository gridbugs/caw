use caw_midi_udp_client::*;
use caw_widgets::Knob;
use clap::{Parser, Subcommand};

#[derive(Subcommand)]
enum Command {
    Knob {
        #[arg(short, long)]
        server: String,
        #[arg(long, default_value_t = 0)]
        channel: u8,
        #[arg(short, long, default_value_t = 0)]
        controller: u8,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long, default_value_t = 0.5)]
        initial_value: f32,
        #[arg(long, default_value_t = 0.2)]
        sensitivity: f32,
    },
}

#[derive(Parser)]
#[command(name = "caw_midi_udp_widgets_app")]
#[command(
    about = "App for launching widgets that communicate with a caw synthesizer by sending midi commands over UDP"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

fn main() {
    match Cli::parse().command {
        Command::Knob {
            server,
            channel,
            controller,
            title,
            initial_value,
            sensitivity,
        } => {
            let client = MidiUdpClient::new(server).unwrap();
            let title = title.unwrap_or_else(|| "".to_string());
            let mut knob =
                Knob::new(title.as_str(), initial_value, sensitivity).unwrap();
            loop {
                knob.tick().unwrap();
                client
                    .send(MidiEvent {
                        channel: channel.into(),
                        message: MidiMessage::Controller {
                            controller: controller.into(),
                            value: knob.value_midi(),
                        },
                    })
                    .unwrap();
            }
        }
    }
}
