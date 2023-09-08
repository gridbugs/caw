use crate::signal::Freq;

#[derive(Debug, Clone, Copy)]
pub enum NoteName {
    C,
    CSharp,
    D,
    DSharp,
    E,
    F,
    FSharp,
    G,
    GSharp,
    A,
    ASharp,
    B,
}

use NoteName::*;
const NOTES_PER_OCTAVE: u8 = 12;
const ALL_NOTE_NAMES_IN_INDEX_ORDER: [NoteName; NOTES_PER_OCTAVE as usize] =
    [C, CSharp, D, DSharp, E, F, FSharp, G, GSharp, A, ASharp, B];

impl NoteName {
    fn to_index(self) -> u8 {
        match self {
            C => 0,
            CSharp => 1,
            D => 2,
            DSharp => 3,
            E => 4,
            F => 5,
            FSharp => 6,
            G => 7,
            GSharp => 8,
            A => 9,
            ASharp => 10,
            B => 11,
        }
    }
}

const A4_FREQ_HZ: f64 = 440.0;
const A4_MIDI_INDEX: u8 = 57;

pub fn freq_hz_of_midi_index(midi_index: u8) -> f64 {
    A4_FREQ_HZ
        * (2_f64.powf((midi_index as f64 - A4_MIDI_INDEX as f64) / (NOTES_PER_OCTAVE as f64)))
}

/// Definition of notes based on MIDI tuned to A440
pub struct Note {
    pub name: NoteName,
    pub octave: u8,
}

impl Note {
    pub const fn new(name: NoteName, octave: u8) -> Self {
        Self { name, octave }
    }

    pub fn to_midi_index(self) -> u8 {
        (self.octave * NOTES_PER_OCTAVE) + self.name.to_index()
    }

    pub fn freq_hz(self) -> f64 {
        freq_hz_of_midi_index(self.to_midi_index())
    }

    pub fn freq(self) -> Freq {
        Freq::from_hz(self.freq_hz())
    }

    pub const fn from_midi_index(midi_index: u8) -> Self {
        let name = ALL_NOTE_NAMES_IN_INDEX_ORDER[(midi_index % NOTES_PER_OCTAVE) as usize];
        let octave = midi_index / NOTES_PER_OCTAVE;
        Self::new(name, octave)
    }
}
