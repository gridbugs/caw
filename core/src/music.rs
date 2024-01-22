use crate::signal::Freq;

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
const A4_MIDI_INDEX: u8 = 57;

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
            midi_index: (octave * NOTES_PER_OCTAVE) + name.to_index(),
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
