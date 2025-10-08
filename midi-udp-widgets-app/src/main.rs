//! An executable which displays a widget and runs a UDP client which sends MIDI messages over a
//! UDP socket to a server (specified as a command-line argument). A caw synthesizer is expected to
//! run a UDP server which receives MIDI over UDP.

use std::time::{Duration, Instant};

use caw_keyboard::{Note, note};
use caw_midi::MidiEvent;
use caw_midi_udp_client::*;
use caw_widgets::{
    AxisLabels, Button, ComputerKeyboard, Knob, NumKeysBits7, Switch, Xy,
};
use clap::{Parser, Subcommand};
use midly::num::u7;

#[derive(Subcommand)]
enum Command {
    Button {
        /// Buttons are implemented with midi key presses. The note determines which key will
        /// represent this button.
        #[arg(long)]
        note: Note,
        #[arg(long)]
        space_channel: Option<u8>,
        #[arg(long)]
        space_controller: Option<u8>,
    },
    Switch {
        /// Switches are implemented with midi key presses. The note determines which key will
        /// represent this switch.
        #[arg(long)]
        note: Note,
    },
    Knob {
        #[arg(short, long, default_value_t = 0)]
        controller: u8,
        #[arg(short, long, default_value_t = 0.5)]
        initial_value: f32,
        #[arg(long, default_value_t = 0.2)]
        sensitivity: f32,
        #[arg(long)]
        space_channel: Option<u8>,
        #[arg(long)]
        space_controller: Option<u8>,
    },
    Xy {
        #[arg(long, default_value_t = 0)]
        controller_x: u8,
        #[arg(long, default_value_t = 1)]
        controller_y: u8,
        #[arg(long)]
        axis_label_x: Option<String>,
        #[arg(long)]
        axis_label_y: Option<String>,
        #[arg(long)]
        space_channel: Option<u8>,
        #[arg(long)]
        space_controller: Option<u8>,
    },
    ComputerKeyboard {
        #[arg(long, default_value_t = note::B_2)]
        start_note: Note,
    },
    NumKeysBits7 {
        #[arg(short, long, default_value_t = 0)]
        controller: u8,
        #[arg(long)]
        space_channel: Option<u8>,
        #[arg(long)]
        space_controller: Option<u8>,
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

/// How long to spam the connection before switching to only sending updates when a value changes.
/// This allows the widget to be created before the server is ready to start receiving messages
/// without the server missing the initial state message as long as the server becomes ready within
/// this duration..
const SPAM_DURATION: Duration = Duration::from_millis(1000);

fn main() {
    let cli = Cli::parse();
    let client = MidiUdpClient::new(cli.server).unwrap();
    let channel: u4 = cli.channel.into();
    let spam_end = Instant::now() + SPAM_DURATION;
    let is_spam = || Instant::now() < spam_end;
    match cli.command {
        Command::Button {
            note,
            space_channel,
            space_controller,
        } => {
            let mut button = Button::new(cli.title.as_deref()).unwrap();
            let mut prev_pressed = false;
            let mut prev_space = false;
            loop {
                button.tick().unwrap();
                let pressed = button.pressed();
                if is_spam() || pressed != prev_pressed {
                    let key = note.to_midi_index().into();
                    let message = if button.pressed() {
                        MidiMessage::NoteOn { key, vel: 0.into() }
                    } else {
                        MidiMessage::NoteOff { key, vel: 0.into() }
                    };
                    client.send(MidiEvent { channel, message }).unwrap();
                    prev_pressed = pressed;
                }
                let space = button.is_space_pressed();
                if let Some(space_controller) = space_controller {
                    if is_spam() || space != prev_space {
                        client
                            .send(MidiEvent {
                                channel: space_channel
                                    .map(|x| x.into())
                                    .unwrap_or(channel),
                                message: MidiMessage::Controller {
                                    controller: space_controller.into(),
                                    value: if space {
                                        u7::max_value()
                                    } else {
                                        0.into()
                                    },
                                },
                            })
                            .unwrap();
                        prev_space = space;
                    }
                }
            }
        }
        Command::Switch { note } => {
            let mut button = Switch::new(cli.title.as_deref()).unwrap();
            let mut prev_pressed = false;
            loop {
                button.tick().unwrap();
                let pressed = button.pressed();
                if is_spam() || pressed != prev_pressed {
                    let key = note.to_midi_index().into();
                    let message = if button.pressed() {
                        MidiMessage::NoteOn { key, vel: 0.into() }
                    } else {
                        MidiMessage::NoteOff { key, vel: 0.into() }
                    };
                    client.send(MidiEvent { channel, message }).unwrap();
                    prev_pressed = pressed;
                }
            }
        }
        Command::Knob {
            controller,
            initial_value,
            sensitivity,
            space_channel,
            space_controller,
        } => {
            let mut knob =
                Knob::new(cli.title.as_deref(), initial_value, sensitivity)
                    .unwrap();
            let send = |value| {
                client
                    .send(MidiEvent {
                        channel,
                        message: MidiMessage::Controller {
                            controller: controller.into(),
                            value,
                        },
                    })
                    .unwrap();
            };
            let mut prev_value = knob.value_midi();
            send(prev_value);
            let mut prev_space = false;
            loop {
                knob.tick().unwrap();
                let value = knob.value_midi();
                if is_spam() || value != prev_value {
                    send(value);
                    prev_value = value;
                }
                let space = knob.is_space_pressed();
                if let Some(space_controller) = space_controller {
                    if is_spam() || space != prev_space {
                        client
                            .send(MidiEvent {
                                channel: space_channel
                                    .map(|x| x.into())
                                    .unwrap_or(channel),
                                message: MidiMessage::Controller {
                                    controller: space_controller.into(),
                                    value: if space {
                                        u7::max_value()
                                    } else {
                                        0.into()
                                    },
                                },
                            })
                            .unwrap();
                        prev_space = space;
                    }
                }
            }
        }
        Command::Xy {
            controller_x,
            controller_y,
            axis_label_x,
            axis_label_y,
            space_channel,
            space_controller,
        } => {
            let axis_labels = AxisLabels {
                x: axis_label_x,
                y: axis_label_y,
            };
            let mut xy = Xy::new(cli.title.as_deref(), axis_labels).unwrap();
            let send_x = |value| {
                client
                    .send(MidiEvent {
                        channel,
                        message: MidiMessage::Controller {
                            controller: controller_x.into(),
                            value,
                        },
                    })
                    .unwrap();
            };
            let send_y = |value| {
                client
                    .send(MidiEvent {
                        channel,
                        message: MidiMessage::Controller {
                            controller: controller_y.into(),
                            value,
                        },
                    })
                    .unwrap();
            };
            xy.tick().unwrap();
            let (mut prev_x, mut prev_y) = xy.value_midi();
            send_x(prev_x);
            send_y(prev_y);
            let mut prev_space = false;
            loop {
                xy.tick().unwrap();
                let (x, y) = xy.value_midi();
                if is_spam() || x != prev_x {
                    send_x(x);
                    prev_x = x;
                }
                if is_spam() || y != prev_y {
                    send_y(y);
                    prev_y = y;
                }
                let space = xy.is_space_pressed();
                if let Some(space_controller) = space_controller {
                    if is_spam() || space != prev_space {
                        client
                            .send(MidiEvent {
                                channel: space_channel
                                    .map(|x| x.into())
                                    .unwrap_or(channel),
                                message: MidiMessage::Controller {
                                    controller: space_controller.into(),
                                    value: if space {
                                        u7::max_value()
                                    } else {
                                        0.into()
                                    },
                                },
                            })
                            .unwrap();

                        prev_space = space;
                    }
                }
            }
        }
        Command::ComputerKeyboard { start_note } => {
            let mut buf = Vec::new();
            let mut computer_keyboard =
                ComputerKeyboard::new(cli.title.as_deref(), start_note)
                    .unwrap();
            loop {
                computer_keyboard.tick(&mut buf).unwrap();
                for message in buf.drain(..) {
                    let midi_event = MidiEvent { channel, message };
                    client.send(midi_event).unwrap();
                }
            }
        }
        Command::NumKeysBits7 {
            controller,
            space_channel,
            space_controller,
        } => {
            let mut num_keys_bits_7 =
                NumKeysBits7::new(cli.title.as_deref()).unwrap();
            num_keys_bits_7.tick().unwrap();
            let send = |value| {
                client
                    .send(MidiEvent {
                        channel: space_channel
                            .map(|x| x.into())
                            .unwrap_or(channel),
                        message: MidiMessage::Controller {
                            controller: controller.into(),
                            value,
                        },
                    })
                    .unwrap()
            };
            let mut prev = num_keys_bits_7.value();
            let mut prev_space = false;
            loop {
                num_keys_bits_7.tick().unwrap();
                let value = num_keys_bits_7.value();
                if is_spam() || value != prev {
                    send(value);
                    prev = value;
                }
                let space = num_keys_bits_7.is_space_pressed();
                if let Some(space_controller) = space_controller {
                    if is_spam() || space != prev_space {
                        client
                            .send(MidiEvent {
                                channel,
                                message: MidiMessage::Controller {
                                    controller: space_controller.into(),
                                    value: if space {
                                        u7::max_value()
                                    } else {
                                        0.into()
                                    },
                                },
                            })
                            .unwrap();

                        prev_space = space;
                    }
                }
            }
        }
    }
}
