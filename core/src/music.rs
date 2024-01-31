use crate::signal::{Freq, Sf64, Sfreq, Signal};

/// A note without an octave
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct NoteName {
    relative_midi_index: u8,
}

const NOTES_PER_OCTAVE: u8 = 12;

impl NoteName {
    const fn from_index(relative_midi_index: u8) -> Self {
        assert!(relative_midi_index < NOTES_PER_OCTAVE);
        Self {
            relative_midi_index,
        }
    }

    pub const C: Self = Self::from_index(0);
    pub const C_SHARP: Self = Self::from_index(1);
    pub const D: Self = Self::from_index(2);
    pub const D_SHARP: Self = Self::from_index(3);
    pub const E: Self = Self::from_index(4);
    pub const F: Self = Self::from_index(5);
    pub const F_SHARP: Self = Self::from_index(6);
    pub const G: Self = Self::from_index(7);
    pub const G_SHARP: Self = Self::from_index(8);
    pub const A: Self = Self::from_index(9);
    pub const A_SHARP: Self = Self::from_index(10);
    pub const B: Self = Self::from_index(11);

    const fn to_index(self) -> u8 {
        self.relative_midi_index
    }

    const fn wrapping_add_semitones(self, num_semitones: i8) -> Self {
        Self::from_index(
            (self.to_index() as i8 + num_semitones).rem_euclid(NOTES_PER_OCTAVE as i8) as u8,
        )
    }

    pub const fn in_octave(self, octave: u8) -> Note {
        Note::new(self, octave)
    }
}

/// Duplicated from `NoteName` so it's possible to bring all note names into scope by using this
/// module.
pub mod note_name {
    pub use super::NoteName;
    pub const C: NoteName = NoteName::C;
    pub const C_SHARP: NoteName = NoteName::C_SHARP;
    pub const D: NoteName = NoteName::D;
    pub const D_SHARP: NoteName = NoteName::D_SHARP;
    pub const E: NoteName = NoteName::E;
    pub const F: NoteName = NoteName::F;
    pub const F_SHARP: NoteName = NoteName::F_SHARP;
    pub const G: NoteName = NoteName::G;
    pub const G_SHARP: NoteName = NoteName::G_SHARP;
    pub const A: NoteName = NoteName::A;
    pub const A_SHARP: NoteName = NoteName::A_SHARP;
    pub const B: NoteName = NoteName::B;
}

const A4_FREQ_HZ: f64 = 440.0;
const C0_MIDI_INDEX: u8 = 12;
const A4_MIDI_INDEX: u8 = C0_MIDI_INDEX + 57;

pub fn freq_hz_of_midi_index(midi_index: u8) -> f64 {
    A4_FREQ_HZ
        * (2_f64.powf((midi_index as f64 - A4_MIDI_INDEX as f64) / (NOTES_PER_OCTAVE as f64)))
}

pub fn semitone_ratio(num_semitones: f64) -> f64 {
    2.0_f64.powf(num_semitones / (NOTES_PER_OCTAVE as f64))
}

/// Definition of notes based on MIDI tuned to A440
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Note {
    midi_index: u8,
}

impl Note {
    pub const fn new(name: NoteName, octave: u8) -> Self {
        Self {
            midi_index: C0_MIDI_INDEX + (octave * NOTES_PER_OCTAVE) + name.to_index(),
        }
    }

    pub fn to_midi_index(self) -> u8 {
        self.midi_index
    }

    pub fn freq_hz(self) -> f64 {
        freq_hz_of_midi_index(self.to_midi_index())
    }

    pub fn freq(self) -> Freq {
        Freq::from_hz(self.freq_hz())
    }

    pub fn from_midi_index(midi_index: impl Into<u8>) -> Self {
        Self {
            midi_index: midi_index.into(),
        }
    }

    pub fn octave(self) -> u8 {
        self.midi_index / NOTES_PER_OCTAVE
    }

    pub fn add_semitones(self, num_semitones: i16) -> Self {
        Self {
            midi_index: (self.midi_index as i16 + num_semitones) as u8,
        }
    }

    pub fn add_octaves(self, num_octaves: i8) -> Self {
        self.add_semitones(num_octaves as i16 * NOTES_PER_OCTAVE as i16)
    }
}

/// Returns the note C4. This is only `Default` so that a `Signal<Note>` can be constructed.
impl Default for Note {
    fn default() -> Self {
        Self::C4
    }
}

impl Signal<Note> {
    pub fn freq(&self) -> Sfreq {
        self.map(|note| note.freq())
    }

    pub fn freq_hz(&self) -> Sf64 {
        self.map(|note| note.freq_hz())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ThirdMod {
    Major,
    Minor,
    Open,
    Sus2,
    Sus4,
    Sus24,
}

#[derive(Clone, Copy, Debug)]
pub struct AbstractChord {
    pub root: NoteName,
    pub third_mod: ThirdMod,
    pub diminished: bool,
}

impl AbstractChord {
    pub const fn major(root: NoteName) -> Self {
        Self {
            root,
            third_mod: ThirdMod::Major,
            diminished: false,
        }
    }

    pub const fn minor(root: NoteName) -> Self {
        Self {
            root,
            third_mod: ThirdMod::Minor,
            diminished: false,
        }
    }

    pub const fn diminished(root: NoteName) -> Self {
        Self {
            root,
            third_mod: ThirdMod::Minor,
            diminished: true,
        }
    }

    pub const fn sus2(root: NoteName) -> Self {
        Self {
            root,
            third_mod: ThirdMod::Sus2,
            diminished: false,
        }
    }

    pub const fn sus4(root: NoteName) -> Self {
        Self {
            root,
            third_mod: ThirdMod::Sus4,
            diminished: false,
        }
    }

    pub fn with_note_names<F: FnMut(NoteName)>(self, mut f: F) {
        f(self.root);
        use ThirdMod::*;
        match self.third_mod {
            Major => f(self.root.wrapping_add_semitones(4)),
            Minor => f(self.root.wrapping_add_semitones(3)),
            Open => (),
            Sus2 => f(self.root.wrapping_add_semitones(2)),
            Sus4 => f(self.root.wrapping_add_semitones(5)),
            Sus24 => {
                f(self.root.wrapping_add_semitones(2));
                f(self.root.wrapping_add_semitones(5));
            }
        }
        if self.diminished {
            f(self.root.wrapping_add_semitones(6));
        } else {
            f(self.root.wrapping_add_semitones(7));
        }
    }

    pub fn with_notes_in_octave<F: FnMut(Note)>(self, octave_base: Note, mut f: F) {
        let octave_base_index = octave_base.to_midi_index() as i16;
        let multiple_of_12_gte_base_index_delta = if octave_base_index == 0 {
            0
        } else {
            ((((octave_base_index - 1) / 12) + 1) * 12) - octave_base_index
        } as i8;
        assert!(multiple_of_12_gte_base_index_delta < 12);
        assert!(multiple_of_12_gte_base_index_delta >= 0);
        self.with_note_names(|note_name| {
            f(Note::from_midi_index(
                octave_base.to_midi_index()
                    + note_name
                        .wrapping_add_semitones(multiple_of_12_gte_base_index_delta)
                        .to_index(),
            ));
        });
    }

    pub fn notes_in_octave(self, octave_base: Note) -> Vec<Note> {
        let mut ret = Vec::new();
        self.with_notes_in_octave(octave_base, |note| ret.push(note));
        ret
    }
}

impl Note {
    pub const C0: Self = Self::new(NoteName::C, 0);
    pub const C1: Self = Self::new(NoteName::C, 1);
    pub const C2: Self = Self::new(NoteName::C, 2);
    pub const C3: Self = Self::new(NoteName::C, 3);
    pub const C4: Self = Self::new(NoteName::C, 4);
    pub const C5: Self = Self::new(NoteName::C, 5);
    pub const C6: Self = Self::new(NoteName::C, 6);
    pub const C7: Self = Self::new(NoteName::C, 7);
    pub const C8: Self = Self::new(NoteName::C, 8);
    pub const D0: Self = Self::new(NoteName::D, 0);
    pub const D1: Self = Self::new(NoteName::D, 1);
    pub const D2: Self = Self::new(NoteName::D, 2);
    pub const D3: Self = Self::new(NoteName::D, 3);
    pub const D4: Self = Self::new(NoteName::D, 4);
    pub const D5: Self = Self::new(NoteName::D, 5);
    pub const D6: Self = Self::new(NoteName::D, 6);
    pub const D7: Self = Self::new(NoteName::D, 7);
    pub const D8: Self = Self::new(NoteName::D, 8);
    pub const E0: Self = Self::new(NoteName::E, 0);
    pub const E1: Self = Self::new(NoteName::E, 1);
    pub const E2: Self = Self::new(NoteName::E, 2);
    pub const E3: Self = Self::new(NoteName::E, 3);
    pub const E4: Self = Self::new(NoteName::E, 4);
    pub const E5: Self = Self::new(NoteName::E, 5);
    pub const E6: Self = Self::new(NoteName::E, 6);
    pub const E7: Self = Self::new(NoteName::E, 7);
    pub const E8: Self = Self::new(NoteName::E, 8);
    pub const F0: Self = Self::new(NoteName::F, 0);
    pub const F1: Self = Self::new(NoteName::F, 1);
    pub const F2: Self = Self::new(NoteName::F, 2);
    pub const F3: Self = Self::new(NoteName::F, 3);
    pub const F4: Self = Self::new(NoteName::F, 4);
    pub const F5: Self = Self::new(NoteName::F, 5);
    pub const F6: Self = Self::new(NoteName::F, 6);
    pub const F7: Self = Self::new(NoteName::F, 7);
    pub const F8: Self = Self::new(NoteName::F, 8);
    pub const G0: Self = Self::new(NoteName::G, 0);
    pub const G1: Self = Self::new(NoteName::G, 1);
    pub const G2: Self = Self::new(NoteName::G, 2);
    pub const G3: Self = Self::new(NoteName::G, 3);
    pub const G4: Self = Self::new(NoteName::G, 4);
    pub const G5: Self = Self::new(NoteName::G, 5);
    pub const G6: Self = Self::new(NoteName::G, 6);
    pub const G7: Self = Self::new(NoteName::G, 7);
    pub const G8: Self = Self::new(NoteName::G, 8);
    pub const A0: Self = Self::new(NoteName::A, 0);
    pub const A1: Self = Self::new(NoteName::A, 1);
    pub const A2: Self = Self::new(NoteName::A, 2);
    pub const A3: Self = Self::new(NoteName::A, 3);
    pub const A4: Self = Self::new(NoteName::A, 4);
    pub const A5: Self = Self::new(NoteName::A, 5);
    pub const A6: Self = Self::new(NoteName::A, 6);
    pub const A7: Self = Self::new(NoteName::A, 7);
    pub const A8: Self = Self::new(NoteName::A, 8);
    pub const B0: Self = Self::new(NoteName::B, 0);
    pub const B1: Self = Self::new(NoteName::B, 1);
    pub const B2: Self = Self::new(NoteName::B, 2);
    pub const B3: Self = Self::new(NoteName::B, 3);
    pub const B4: Self = Self::new(NoteName::B, 4);
    pub const B5: Self = Self::new(NoteName::B, 5);
    pub const B6: Self = Self::new(NoteName::B, 6);
    pub const B7: Self = Self::new(NoteName::B, 7);
    pub const B8: Self = Self::new(NoteName::B, 8);
}

/// Duplicated from `Note` so it's possible to bring all notes into scope by using this module.
pub mod note {
    pub use super::Note;
    pub const C0: Note = Note::C0;
    pub const C1: Note = Note::C1;
    pub const C2: Note = Note::C2;
    pub const C3: Note = Note::C3;
    pub const C4: Note = Note::C4;
    pub const C5: Note = Note::C5;
    pub const C6: Note = Note::C6;
    pub const C7: Note = Note::C7;
    pub const C8: Note = Note::C8;
    pub const D0: Note = Note::D0;
    pub const D1: Note = Note::D1;
    pub const D2: Note = Note::D2;
    pub const D3: Note = Note::D3;
    pub const D4: Note = Note::D4;
    pub const D5: Note = Note::D5;
    pub const D6: Note = Note::D6;
    pub const D7: Note = Note::D7;
    pub const D8: Note = Note::D8;
    pub const E0: Note = Note::E0;
    pub const E1: Note = Note::E1;
    pub const E2: Note = Note::E2;
    pub const E3: Note = Note::E3;
    pub const E4: Note = Note::E4;
    pub const E5: Note = Note::E5;
    pub const E6: Note = Note::E6;
    pub const E7: Note = Note::E7;
    pub const E8: Note = Note::E8;
    pub const F0: Note = Note::F0;
    pub const F1: Note = Note::F1;
    pub const F2: Note = Note::F2;
    pub const F3: Note = Note::F3;
    pub const F4: Note = Note::F4;
    pub const F5: Note = Note::F5;
    pub const F6: Note = Note::F6;
    pub const F7: Note = Note::F7;
    pub const F8: Note = Note::F8;
    pub const G0: Note = Note::G0;
    pub const G1: Note = Note::G1;
    pub const G2: Note = Note::G2;
    pub const G3: Note = Note::G3;
    pub const G4: Note = Note::G4;
    pub const G5: Note = Note::G5;
    pub const G6: Note = Note::G6;
    pub const G7: Note = Note::G7;
    pub const G8: Note = Note::G8;
    pub const A0: Note = Note::A0;
    pub const A1: Note = Note::A1;
    pub const A2: Note = Note::A2;
    pub const A3: Note = Note::A3;
    pub const A4: Note = Note::A4;
    pub const A5: Note = Note::A5;
    pub const A6: Note = Note::A6;
    pub const A7: Note = Note::A7;
    pub const A8: Note = Note::A8;
    pub const B0: Note = Note::B0;
    pub const B1: Note = Note::B1;
    pub const B2: Note = Note::B2;
    pub const B3: Note = Note::B3;
    pub const B4: Note = Note::B4;
    pub const B5: Note = Note::B5;
    pub const B6: Note = Note::B6;
    pub const B7: Note = Note::B7;
    pub const B8: Note = Note::B8;
}
