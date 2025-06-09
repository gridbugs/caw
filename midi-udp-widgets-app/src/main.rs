use caw_keyboard::Note;
use caw_midi_udp_client::*;
use caw_widgets::{Button, Knob, Xy};
use clap::{Parser, Subcommand};

#[derive(Subcommand)]
enum Command {
    Button,
    Knob {
        #[arg(short, long, default_value_t = 0)]
        controller: u8,
        #[arg(short, long, default_value_t = 0.5)]
        initial_value: f32,
        #[arg(long, default_value_t = 0.2)]
        sensitivity: f32,
    },
    Xy {
        #[arg(long, default_value_t = 0)]
        controller_x: u8,
        #[arg(long, default_value_t = 1)]
        controller_y: u8,
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
    #[arg(short, long)]
    server: String,
    #[arg(long, default_value_t = 0)]
    channel: u8,
    #[arg(short, long)]
    title: Option<String>,
}

fn main() {
    let cli = Cli::parse();
    let client = MidiUdpClient::new(cli.server).unwrap();
    let channel: u4 = cli.channel.into();
    match cli.command {
        Command::Button => {
            let mut button = Button::new(cli.title.as_deref()).unwrap();
            loop {
                button.tick().unwrap();
                let key = Note::C4.to_midi_index().into();
                let message = if button.pressed() {
                    MidiMessage::NoteOn { key, vel: 0.into() }
                } else {
                    MidiMessage::NoteOff { key, vel: 0.into() }
                };
                client.send(MidiEvent { channel, message }).unwrap();
            }
        }
        Command::Knob {
            controller,
            initial_value,
            sensitivity,
        } => {
            let mut knob =
                Knob::new(cli.title.as_deref(), initial_value, sensitivity)
                    .unwrap();
            loop {
                knob.tick().unwrap();
                client
                    .send(MidiEvent {
                        channel,
                        message: MidiMessage::Controller {
                            controller: controller.into(),
                            value: knob.value_midi(),
                        },
                    })
                    .unwrap();
            }
        }
        Command::Xy {
            controller_x,
            controller_y,
        } => {
            let mut xy = Xy::new(cli.title.as_deref()).unwrap();
            loop {
                xy.tick().unwrap();
                let (x, y) = xy.value_midi();
                client
                    .send(MidiEvent {
                        channel,
                        message: MidiMessage::Controller {
                            controller: controller_x.into(),
                            value: x,
                        },
                    })
                    .unwrap();
                client
                    .send(MidiEvent {
                        channel,
                        message: MidiMessage::Controller {
                            controller: controller_y.into(),
                            value: y,
                        },
                    })
                    .unwrap();
            }
        }
    }
}
