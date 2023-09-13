use clap::Parser;
use midly::{Smf, TrackEventKind};
use std::fs;

#[derive(Parser, Debug)]
struct Args {
    name: String,
}

fn main() {
    let args = Args::parse();
    let data = fs::read(args.name).unwrap();
    let midi = Smf::parse(&data).unwrap();
    println!("{:?}", midi.header);
    println!("num tracks: {}", midi.tracks.len());
    if let Some(track) = midi.tracks.first() {
        for event in track {
            if let TrackEventKind::Midi { channel, message } = event.kind {
                println!("delta: {}", event.delta);
                println!("channel: {}", channel);
                println!("{:?}", message);
            }
        }
    }
    println!("{:?}", midi.header);
}
