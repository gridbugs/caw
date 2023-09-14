use clap::Parser;
use ibis_interactive::midi::MidiFile;

#[derive(Parser, Debug)]
struct Args {
    name: String,
}

fn main() {
    let args = Args::parse();
    let midi_file = MidiFile::read(args.name).unwrap();
    let track = midi_file.track(0).unwrap();
    println!("{:#?}", track);
}
