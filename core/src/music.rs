use crate::signal::{Freq, Sf64, Sfreq, Signal};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Octave(u8);

impl Octave {
    pub const MAX_OCTAVE: u8 = 8;

    pub fn new(i: u8) -> Self {
        assert!(i <= Self::MAX_OCTAVE);
        Self(i)
    }

    pub const OCTAVE_0: Self = Self(0);
    pub const OCTAVE_1: Self = Self(1);
    pub const OCTAVE_2: Self = Self(2);
    pub const OCTAVE_3: Self = Self(3);
    pub const OCTAVE_4: Self = Self(4);
    pub const OCTAVE_5: Self = Self(5);
    pub const OCTAVE_6: Self = Self(6);
    pub const OCTAVE_7: Self = Self(7);
    pub const OCTAVE_8: Self = Self(8);
}

impl Default for Octave {
    fn default() -> Self {
        Octave::OCTAVE_4
    }
}

pub mod octave {
    use super::Octave;

    pub const OCTAVE_0: Octave = Octave::OCTAVE_0;
    pub const OCTAVE_1: Octave = Octave::OCTAVE_1;
    pub const OCTAVE_2: Octave = Octave::OCTAVE_2;
    pub const OCTAVE_3: Octave = Octave::OCTAVE_3;
    pub const OCTAVE_4: Octave = Octave::OCTAVE_4;
    pub const OCTAVE_5: Octave = Octave::OCTAVE_5;
    pub const OCTAVE_6: Octave = Octave::OCTAVE_6;
    pub const OCTAVE_7: Octave = Octave::OCTAVE_7;
    pub const OCTAVE_8: Octave = Octave::OCTAVE_8;
}

/// A note without an octave
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct NoteName {
    relative_midi_index: u8,
}

const NOTES_PER_OCTAVE: u8 = 12;
const MAX_MIDI_INDEX: u8 = 127;

impl NoteName {
    const fn from_index(relative_midi_index: u8) -> Self {
        assert!(relative_midi_index < NOTES_PER_OCTAVE);
        Self {
            relative_midi_index,
        }
    }

    pub const C: Self = Self::from_index(0);
    pub const C_SHARP: Self = Self::from_index(1);
    pub const D_FLAT: Self = Self::C_SHARP;
    pub const D: Self = Self::from_index(2);
    pub const D_SHARP: Self = Self::from_index(3);
    pub const E_FLAT: Self = Self::D_SHARP;
    pub const E: Self = Self::from_index(4);
    pub const F: Self = Self::from_index(5);
    pub const F_SHARP: Self = Self::from_index(6);
    pub const G_FLAT: Self = Self::F_SHARP;
    pub const G: Self = Self::from_index(7);
    pub const G_SHARP: Self = Self::from_index(8);
    pub const A_FLAT: Self = Self::G_SHARP;
    pub const A: Self = Self::from_index(9);
    pub const A_SHARP: Self = Self::from_index(10);
    pub const B_FLAT: Self = Self::A_SHARP;
    pub const B: Self = Self::from_index(11);

    const fn to_index(self) -> u8 {
        self.relative_midi_index
    }

    const fn wrapping_add_semitones(self, num_semitones: i8) -> Self {
        Self::from_index(
            (self.to_index() as i8 + num_semitones).rem_euclid(NOTES_PER_OCTAVE as i8) as u8,
        )
    }

    pub const fn in_octave(self, octave: Octave) -> Note {
        Note::new(self, octave)
    }
}

/// Duplicated from `NoteName` so it's possible to bring all note names into scope by using this
/// module.
pub mod note_name {
    pub use super::NoteName;
    pub const C: NoteName = NoteName::C;
    pub const C_SHARP: NoteName = NoteName::C_SHARP;
    pub const D_FLAT: NoteName = NoteName::C_SHARP;
    pub const D: NoteName = NoteName::D;
    pub const D_SHARP: NoteName = NoteName::D_SHARP;
    pub const E_FLAT: NoteName = NoteName::D_SHARP;
    pub const E: NoteName = NoteName::E;
    pub const F: NoteName = NoteName::F;
    pub const F_SHARP: NoteName = NoteName::F_SHARP;
    pub const G_FLAT: NoteName = NoteName::F_SHARP;
    pub const G: NoteName = NoteName::G;
    pub const G_SHARP: NoteName = NoteName::G_SHARP;
    pub const A_FLAT: NoteName = NoteName::G_SHARP;
    pub const A: NoteName = NoteName::A;
    pub const A_SHARP: NoteName = NoteName::A_SHARP;
    pub const B_FLAT: NoteName = NoteName::A_SHARP;
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

pub const TONE_RATIO: f64 = 1.122462048309373;

/// Definition of notes based on MIDI tuned to A440
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Note {
    midi_index: u8,
}

impl Note {
    pub const fn new(name: NoteName, octave: Octave) -> Self {
        Self {
            midi_index: C0_MIDI_INDEX + (octave.0 * NOTES_PER_OCTAVE) + name.to_index(),
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

    pub fn octave(self) -> Octave {
        Octave::new(self.midi_index / NOTES_PER_OCTAVE)
    }

    pub fn add_semitones_checked(self, num_semitones: i16) -> Option<Self> {
        let midi_index = self.midi_index as i16 + num_semitones;
        if midi_index < 0 || midi_index > MAX_MIDI_INDEX as i16 {
            None
        } else {
            Some(Self {
                midi_index: midi_index as u8,
            })
        }
    }

    pub fn add_octaves_checked(self, num_octaves: i8) -> Option<Self> {
        self.add_semitones_checked(num_octaves as i16 * NOTES_PER_OCTAVE as i16)
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

pub mod chord {
    use super::{Note, NoteName, Octave};

    #[derive(Clone, Copy, Debug)]
    pub enum Third {
        Major,
        Minor,
        Sus2,
        Sus4,
    }

    #[derive(Clone, Copy, Debug)]
    pub enum Fifth {
        Perfect,
        Diminished,
    }

    #[derive(Clone, Copy, Debug)]
    pub enum Seventh {
        Major,
        Minor,
    }

    #[derive(Clone, Copy, Debug)]
    pub struct ChordType {
        pub third: Option<Third>,
        pub fifth: Option<Fifth>,
        pub seventh: Option<Seventh>,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ChordPosition {
        Root,
        Third,
        Fifth,
        Seventh,
    }

    impl ChordType {
        pub const fn major_7(self) -> Self {
            Self {
                seventh: Some(Seventh::Major),
                ..self
            }
        }

        pub const fn minor_7(self) -> Self {
            Self {
                seventh: Some(Seventh::Minor),
                ..self
            }
        }

        pub const fn infer_7(self) -> Self {
            let seventh = Some(match self.third {
                Some(Third::Minor) => Seventh::Minor,
                _ => Seventh::Major,
            });
            Self { seventh, ..self }
        }

        pub const fn flat_5(self) -> Self {
            Self {
                fifth: Some(Fifth::Diminished),
                ..self
            }
        }

        pub const fn num_notes(&self) -> u8 {
            // start with 1 for the root note which is always present
            1 + self.third.is_some() as u8
                + self.fifth.is_some() as u8
                + self.seventh.is_some() as u8
        }

        pub fn with_semitones_above_root<F: FnMut(i8, ChordPosition)>(&self, mut f: F) {
            f(0, ChordPosition::Root);
            if let Some(third) = self.third {
                match third {
                    Third::Major => f(4, ChordPosition::Third),
                    Third::Minor => f(3, ChordPosition::Third),
                    Third::Sus2 => f(2, ChordPosition::Third),
                    Third::Sus4 => f(5, ChordPosition::Third),
                }
            }
            if let Some(fifth) = self.fifth {
                match fifth {
                    Fifth::Perfect => f(7, ChordPosition::Fifth),
                    Fifth::Diminished => f(6, ChordPosition::Fifth),
                }
            }
            if let Some(seventh) = self.seventh {
                match seventh {
                    Seventh::Major => f(11, ChordPosition::Seventh),
                    Seventh::Minor => f(10, ChordPosition::Seventh),
                }
            }
        }
    }

    pub const MAJOR: ChordType = ChordType {
        third: Some(Third::Major),
        fifth: Some(Fifth::Perfect),
        seventh: None,
    };

    pub const MINOR: ChordType = ChordType {
        third: Some(Third::Minor),
        fifth: Some(Fifth::Perfect),
        seventh: None,
    };

    pub const DIMINISHED: ChordType = ChordType {
        third: Some(Third::Minor),
        fifth: Some(Fifth::Diminished),
        seventh: None,
    };

    pub const SUS_2: ChordType = ChordType {
        third: Some(Third::Sus2),
        fifth: Some(Fifth::Perfect),
        seventh: None,
    };

    pub const SUS_4: ChordType = ChordType {
        third: Some(Third::Sus4),
        fifth: Some(Fifth::Perfect),
        seventh: None,
    };

    pub const OPEN: ChordType = ChordType {
        third: None,
        fifth: Some(Fifth::Perfect),
        seventh: None,
    };

    fn wrap_note_within_octave(octave_base: Note, root: NoteName, semitones_above: i8) -> Note {
        let octave_base_index = octave_base.to_midi_index();
        let multiple_of_12_gte_base_index_delta = if octave_base_index == 0 {
            0
        } else {
            ((((octave_base_index - 1) / 12) + 1) * 12) - octave_base_index
        } as i8;
        let note_name = root.wrapping_add_semitones(semitones_above);
        assert!(multiple_of_12_gte_base_index_delta < 12);
        assert!(multiple_of_12_gte_base_index_delta >= 0);
        Note::from_midi_index(
            octave_base_index
                + note_name
                    .wrapping_add_semitones(multiple_of_12_gte_base_index_delta)
                    .to_index(),
        )
    }

    #[derive(Clone, Copy, Debug)]
    pub enum Inversion {
        WithRootOctave {
            root_octave: Octave,
            lowest_position: ChordPosition,
        },
        InOctave {
            octave_base: Note,
        },
    }

    impl Default for Inversion {
        fn default() -> Self {
            Self::WithRootOctave {
                root_octave: Octave::OCTAVE_4,
                lowest_position: ChordPosition::Root,
            }
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub struct Chord {
        pub root: NoteName,
        pub typ: ChordType,
        pub octave_shift: i8,
    }

    impl Chord {
        pub fn new(root: NoteName, typ: ChordType) -> Self {
            Self {
                root,
                typ,
                octave_shift: 0,
            }
        }

        pub fn octave_shift(self, octave_shift: i8) -> Self {
            Self {
                octave_shift,
                ..self
            }
        }

        fn with_notes_in_octave<F: FnMut(Note)>(self, octave_base: Note, mut f: F) {
            self.typ.with_semitones_above_root(|semitones_above, _| {
                let note = wrap_note_within_octave(octave_base, self.root, semitones_above)
                    .add_octaves(self.octave_shift);
                f(note);
            });
        }

        fn with_notes_root_octave<F: FnMut(Note)>(
            self,
            root_octave: Octave,
            lowest_position: ChordPosition,
            mut f: F,
        ) {
            let root_note = self.root.in_octave(root_octave);
            let mut shifting_down = false;
            self.typ
                .with_semitones_above_root(|semitones_above, chord_position| {
                    let mut note = root_note
                        .add_semitones(semitones_above as i16)
                        .add_octaves(self.octave_shift);
                    if !shifting_down
                        && lowest_position != ChordPosition::Root
                        && lowest_position == chord_position
                    {
                        shifting_down = true;
                    }
                    if shifting_down {
                        note = note.add_octaves(-1);
                    }
                    f(note);
                });
        }

        pub fn with_notes<F: FnMut(Note)>(self, inversion: Inversion, f: F) {
            match inversion {
                Inversion::WithRootOctave {
                    root_octave,
                    lowest_position,
                } => self.with_notes_root_octave(root_octave, lowest_position, f),
                Inversion::InOctave { octave_base } => self.with_notes_in_octave(octave_base, f),
            }
        }

        pub fn notes(self, inversion: Inversion) -> Vec<Note> {
            let mut ret = Vec::new();
            self.with_notes(inversion, |note| {
                ret.push(note);
            });
            ret
        }
    }

    pub fn chord(root: NoteName, typ: ChordType) -> Chord {
        Chord::new(root, typ)
    }
}

impl Note {
    pub const C0: Self = Self::new(NoteName::C, octave::OCTAVE_0);
    pub const C1: Self = Self::new(NoteName::C, octave::OCTAVE_1);
    pub const C2: Self = Self::new(NoteName::C, octave::OCTAVE_2);
    pub const C3: Self = Self::new(NoteName::C, octave::OCTAVE_3);
    pub const C4: Self = Self::new(NoteName::C, octave::OCTAVE_4);
    pub const C5: Self = Self::new(NoteName::C, octave::OCTAVE_5);
    pub const C6: Self = Self::new(NoteName::C, octave::OCTAVE_6);
    pub const C7: Self = Self::new(NoteName::C, octave::OCTAVE_7);
    pub const C8: Self = Self::new(NoteName::C, octave::OCTAVE_8);
    pub const C_SHARP0: Self = Self::new(NoteName::C_SHARP, octave::OCTAVE_0);
    pub const C_SHARP1: Self = Self::new(NoteName::C_SHARP, octave::OCTAVE_1);
    pub const C_SHARP2: Self = Self::new(NoteName::C_SHARP, octave::OCTAVE_2);
    pub const C_SHARP3: Self = Self::new(NoteName::C_SHARP, octave::OCTAVE_3);
    pub const C_SHARP4: Self = Self::new(NoteName::C_SHARP, octave::OCTAVE_4);
    pub const C_SHARP5: Self = Self::new(NoteName::C_SHARP, octave::OCTAVE_5);
    pub const C_SHARP6: Self = Self::new(NoteName::C_SHARP, octave::OCTAVE_6);
    pub const C_SHARP7: Self = Self::new(NoteName::C_SHARP, octave::OCTAVE_7);
    pub const C_SHARP8: Self = Self::new(NoteName::C_SHARP, octave::OCTAVE_8);
    pub const D_FLAT0: Self = Self::new(NoteName::D_FLAT, octave::OCTAVE_0);
    pub const D_FLAT1: Self = Self::new(NoteName::D_FLAT, octave::OCTAVE_1);
    pub const D_FLAT2: Self = Self::new(NoteName::D_FLAT, octave::OCTAVE_2);
    pub const D_FLAT3: Self = Self::new(NoteName::D_FLAT, octave::OCTAVE_3);
    pub const D_FLAT4: Self = Self::new(NoteName::D_FLAT, octave::OCTAVE_4);
    pub const D_FLAT5: Self = Self::new(NoteName::D_FLAT, octave::OCTAVE_5);
    pub const D_FLAT6: Self = Self::new(NoteName::D_FLAT, octave::OCTAVE_6);
    pub const D_FLAT7: Self = Self::new(NoteName::D_FLAT, octave::OCTAVE_7);
    pub const D_FLAT8: Self = Self::new(NoteName::D_FLAT, octave::OCTAVE_8);
    pub const D0: Self = Self::new(NoteName::D, octave::OCTAVE_0);
    pub const D1: Self = Self::new(NoteName::D, octave::OCTAVE_1);
    pub const D2: Self = Self::new(NoteName::D, octave::OCTAVE_2);
    pub const D3: Self = Self::new(NoteName::D, octave::OCTAVE_3);
    pub const D4: Self = Self::new(NoteName::D, octave::OCTAVE_4);
    pub const D5: Self = Self::new(NoteName::D, octave::OCTAVE_5);
    pub const D6: Self = Self::new(NoteName::D, octave::OCTAVE_6);
    pub const D7: Self = Self::new(NoteName::D, octave::OCTAVE_7);
    pub const D8: Self = Self::new(NoteName::D, octave::OCTAVE_8);
    pub const D_SHARP0: Self = Self::new(NoteName::D_SHARP, octave::OCTAVE_0);
    pub const D_SHARP1: Self = Self::new(NoteName::D_SHARP, octave::OCTAVE_1);
    pub const D_SHARP2: Self = Self::new(NoteName::D_SHARP, octave::OCTAVE_2);
    pub const D_SHARP3: Self = Self::new(NoteName::D_SHARP, octave::OCTAVE_3);
    pub const D_SHARP4: Self = Self::new(NoteName::D_SHARP, octave::OCTAVE_4);
    pub const D_SHARP5: Self = Self::new(NoteName::D_SHARP, octave::OCTAVE_5);
    pub const D_SHARP6: Self = Self::new(NoteName::D_SHARP, octave::OCTAVE_6);
    pub const D_SHARP7: Self = Self::new(NoteName::D_SHARP, octave::OCTAVE_7);
    pub const D_SHARP8: Self = Self::new(NoteName::D_SHARP, octave::OCTAVE_8);
    pub const E_FLAT0: Self = Self::new(NoteName::E_FLAT, octave::OCTAVE_0);
    pub const E_FLAT1: Self = Self::new(NoteName::E_FLAT, octave::OCTAVE_1);
    pub const E_FLAT2: Self = Self::new(NoteName::E_FLAT, octave::OCTAVE_2);
    pub const E_FLAT3: Self = Self::new(NoteName::E_FLAT, octave::OCTAVE_3);
    pub const E_FLAT4: Self = Self::new(NoteName::E_FLAT, octave::OCTAVE_4);
    pub const E_FLAT5: Self = Self::new(NoteName::E_FLAT, octave::OCTAVE_5);
    pub const E_FLAT6: Self = Self::new(NoteName::E_FLAT, octave::OCTAVE_6);
    pub const E_FLAT7: Self = Self::new(NoteName::E_FLAT, octave::OCTAVE_7);
    pub const E_FLAT8: Self = Self::new(NoteName::E_FLAT, octave::OCTAVE_8);
    pub const E0: Self = Self::new(NoteName::E, octave::OCTAVE_0);
    pub const E1: Self = Self::new(NoteName::E, octave::OCTAVE_1);
    pub const E2: Self = Self::new(NoteName::E, octave::OCTAVE_2);
    pub const E3: Self = Self::new(NoteName::E, octave::OCTAVE_3);
    pub const E4: Self = Self::new(NoteName::E, octave::OCTAVE_4);
    pub const E5: Self = Self::new(NoteName::E, octave::OCTAVE_5);
    pub const E6: Self = Self::new(NoteName::E, octave::OCTAVE_6);
    pub const E7: Self = Self::new(NoteName::E, octave::OCTAVE_7);
    pub const E8: Self = Self::new(NoteName::E, octave::OCTAVE_8);
    pub const F0: Self = Self::new(NoteName::F, octave::OCTAVE_0);
    pub const F1: Self = Self::new(NoteName::F, octave::OCTAVE_1);
    pub const F2: Self = Self::new(NoteName::F, octave::OCTAVE_2);
    pub const F3: Self = Self::new(NoteName::F, octave::OCTAVE_3);
    pub const F4: Self = Self::new(NoteName::F, octave::OCTAVE_4);
    pub const F5: Self = Self::new(NoteName::F, octave::OCTAVE_5);
    pub const F6: Self = Self::new(NoteName::F, octave::OCTAVE_6);
    pub const F7: Self = Self::new(NoteName::F, octave::OCTAVE_7);
    pub const F8: Self = Self::new(NoteName::F, octave::OCTAVE_8);
    pub const F_SHARP0: Self = Self::new(NoteName::F_SHARP, octave::OCTAVE_0);
    pub const F_SHARP1: Self = Self::new(NoteName::F_SHARP, octave::OCTAVE_1);
    pub const F_SHARP2: Self = Self::new(NoteName::F_SHARP, octave::OCTAVE_2);
    pub const F_SHARP3: Self = Self::new(NoteName::F_SHARP, octave::OCTAVE_3);
    pub const F_SHARP4: Self = Self::new(NoteName::F_SHARP, octave::OCTAVE_4);
    pub const F_SHARP5: Self = Self::new(NoteName::F_SHARP, octave::OCTAVE_5);
    pub const F_SHARP6: Self = Self::new(NoteName::F_SHARP, octave::OCTAVE_6);
    pub const F_SHARP7: Self = Self::new(NoteName::F_SHARP, octave::OCTAVE_7);
    pub const F_SHARP8: Self = Self::new(NoteName::F_SHARP, octave::OCTAVE_8);
    pub const G_FLAT0: Self = Self::new(NoteName::G_FLAT, octave::OCTAVE_0);
    pub const G_FLAT1: Self = Self::new(NoteName::G_FLAT, octave::OCTAVE_1);
    pub const G_FLAT2: Self = Self::new(NoteName::G_FLAT, octave::OCTAVE_2);
    pub const G_FLAT3: Self = Self::new(NoteName::G_FLAT, octave::OCTAVE_3);
    pub const G_FLAT4: Self = Self::new(NoteName::G_FLAT, octave::OCTAVE_4);
    pub const G_FLAT5: Self = Self::new(NoteName::G_FLAT, octave::OCTAVE_5);
    pub const G_FLAT6: Self = Self::new(NoteName::G_FLAT, octave::OCTAVE_6);
    pub const G_FLAT7: Self = Self::new(NoteName::G_FLAT, octave::OCTAVE_7);
    pub const G_FLAT8: Self = Self::new(NoteName::G_FLAT, octave::OCTAVE_8);
    pub const G0: Self = Self::new(NoteName::G, octave::OCTAVE_0);
    pub const G1: Self = Self::new(NoteName::G, octave::OCTAVE_1);
    pub const G2: Self = Self::new(NoteName::G, octave::OCTAVE_2);
    pub const G3: Self = Self::new(NoteName::G, octave::OCTAVE_3);
    pub const G4: Self = Self::new(NoteName::G, octave::OCTAVE_4);
    pub const G5: Self = Self::new(NoteName::G, octave::OCTAVE_5);
    pub const G6: Self = Self::new(NoteName::G, octave::OCTAVE_6);
    pub const G7: Self = Self::new(NoteName::G, octave::OCTAVE_7);
    pub const G8: Self = Self::new(NoteName::G, octave::OCTAVE_8);
    pub const G_SHARP0: Self = Self::new(NoteName::G_SHARP, octave::OCTAVE_0);
    pub const G_SHARP1: Self = Self::new(NoteName::G_SHARP, octave::OCTAVE_1);
    pub const G_SHARP2: Self = Self::new(NoteName::G_SHARP, octave::OCTAVE_2);
    pub const G_SHARP3: Self = Self::new(NoteName::G_SHARP, octave::OCTAVE_3);
    pub const G_SHARP4: Self = Self::new(NoteName::G_SHARP, octave::OCTAVE_4);
    pub const G_SHARP5: Self = Self::new(NoteName::G_SHARP, octave::OCTAVE_5);
    pub const G_SHARP6: Self = Self::new(NoteName::G_SHARP, octave::OCTAVE_6);
    pub const G_SHARP7: Self = Self::new(NoteName::G_SHARP, octave::OCTAVE_7);
    pub const G_SHARP8: Self = Self::new(NoteName::G_SHARP, octave::OCTAVE_8);
    pub const A_FLAT0: Self = Self::new(NoteName::A_FLAT, octave::OCTAVE_0);
    pub const A_FLAT1: Self = Self::new(NoteName::A_FLAT, octave::OCTAVE_1);
    pub const A_FLAT2: Self = Self::new(NoteName::A_FLAT, octave::OCTAVE_2);
    pub const A_FLAT3: Self = Self::new(NoteName::A_FLAT, octave::OCTAVE_3);
    pub const A_FLAT4: Self = Self::new(NoteName::A_FLAT, octave::OCTAVE_4);
    pub const A_FLAT5: Self = Self::new(NoteName::A_FLAT, octave::OCTAVE_5);
    pub const A_FLAT6: Self = Self::new(NoteName::A_FLAT, octave::OCTAVE_6);
    pub const A_FLAT7: Self = Self::new(NoteName::A_FLAT, octave::OCTAVE_7);
    pub const A_FLAT8: Self = Self::new(NoteName::A_FLAT, octave::OCTAVE_8);
    pub const A0: Self = Self::new(NoteName::A, octave::OCTAVE_0);
    pub const A1: Self = Self::new(NoteName::A, octave::OCTAVE_1);
    pub const A2: Self = Self::new(NoteName::A, octave::OCTAVE_2);
    pub const A3: Self = Self::new(NoteName::A, octave::OCTAVE_3);
    pub const A4: Self = Self::new(NoteName::A, octave::OCTAVE_4);
    pub const A5: Self = Self::new(NoteName::A, octave::OCTAVE_5);
    pub const A6: Self = Self::new(NoteName::A, octave::OCTAVE_6);
    pub const A7: Self = Self::new(NoteName::A, octave::OCTAVE_7);
    pub const A8: Self = Self::new(NoteName::A, octave::OCTAVE_8);
    pub const A_SHARP0: Self = Self::new(NoteName::A_SHARP, octave::OCTAVE_0);
    pub const A_SHARP1: Self = Self::new(NoteName::A_SHARP, octave::OCTAVE_1);
    pub const A_SHARP2: Self = Self::new(NoteName::A_SHARP, octave::OCTAVE_2);
    pub const A_SHARP3: Self = Self::new(NoteName::A_SHARP, octave::OCTAVE_3);
    pub const A_SHARP4: Self = Self::new(NoteName::A_SHARP, octave::OCTAVE_4);
    pub const A_SHARP5: Self = Self::new(NoteName::A_SHARP, octave::OCTAVE_5);
    pub const A_SHARP6: Self = Self::new(NoteName::A_SHARP, octave::OCTAVE_6);
    pub const A_SHARP7: Self = Self::new(NoteName::A_SHARP, octave::OCTAVE_7);
    pub const A_SHARP8: Self = Self::new(NoteName::A_SHARP, octave::OCTAVE_8);
    pub const B_FLAT0: Self = Self::new(NoteName::B_FLAT, octave::OCTAVE_0);
    pub const B_FLAT1: Self = Self::new(NoteName::B_FLAT, octave::OCTAVE_1);
    pub const B_FLAT2: Self = Self::new(NoteName::B_FLAT, octave::OCTAVE_2);
    pub const B_FLAT3: Self = Self::new(NoteName::B_FLAT, octave::OCTAVE_3);
    pub const B_FLAT4: Self = Self::new(NoteName::B_FLAT, octave::OCTAVE_4);
    pub const B_FLAT5: Self = Self::new(NoteName::B_FLAT, octave::OCTAVE_5);
    pub const B_FLAT6: Self = Self::new(NoteName::B_FLAT, octave::OCTAVE_6);
    pub const B_FLAT7: Self = Self::new(NoteName::B_FLAT, octave::OCTAVE_7);
    pub const B_FLAT8: Self = Self::new(NoteName::B_FLAT, octave::OCTAVE_8);
    pub const B0: Self = Self::new(NoteName::B, octave::OCTAVE_0);
    pub const B1: Self = Self::new(NoteName::B, octave::OCTAVE_1);
    pub const B2: Self = Self::new(NoteName::B, octave::OCTAVE_2);
    pub const B3: Self = Self::new(NoteName::B, octave::OCTAVE_3);
    pub const B4: Self = Self::new(NoteName::B, octave::OCTAVE_4);
    pub const B5: Self = Self::new(NoteName::B, octave::OCTAVE_5);
    pub const B6: Self = Self::new(NoteName::B, octave::OCTAVE_6);
    pub const B7: Self = Self::new(NoteName::B, octave::OCTAVE_7);
    pub const B8: Self = Self::new(NoteName::B, octave::OCTAVE_8);
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
    pub const C_SHARP0: Note = Note::C_SHARP0;
    pub const C_SHARP1: Note = Note::C_SHARP1;
    pub const C_SHARP2: Note = Note::C_SHARP2;
    pub const C_SHARP3: Note = Note::C_SHARP3;
    pub const C_SHARP4: Note = Note::C_SHARP4;
    pub const C_SHARP5: Note = Note::C_SHARP5;
    pub const C_SHARP6: Note = Note::C_SHARP6;
    pub const C_SHARP7: Note = Note::C_SHARP7;
    pub const C_SHARP8: Note = Note::C_SHARP8;
    pub const D_FLAT0: Note = Note::D_FLAT0;
    pub const D_FLAT1: Note = Note::D_FLAT1;
    pub const D_FLAT2: Note = Note::D_FLAT2;
    pub const D_FLAT3: Note = Note::D_FLAT3;
    pub const D_FLAT4: Note = Note::D_FLAT4;
    pub const D_FLAT5: Note = Note::D_FLAT5;
    pub const D_FLAT6: Note = Note::D_FLAT6;
    pub const D_FLAT7: Note = Note::D_FLAT7;
    pub const D_FLAT8: Note = Note::D_FLAT8;
    pub const D0: Note = Note::D0;
    pub const D1: Note = Note::D1;
    pub const D2: Note = Note::D2;
    pub const D3: Note = Note::D3;
    pub const D4: Note = Note::D4;
    pub const D5: Note = Note::D5;
    pub const D6: Note = Note::D6;
    pub const D7: Note = Note::D7;
    pub const D8: Note = Note::D8;
    pub const D_SHARP0: Note = Note::D_SHARP0;
    pub const D_SHARP1: Note = Note::D_SHARP1;
    pub const D_SHARP2: Note = Note::D_SHARP2;
    pub const D_SHARP3: Note = Note::D_SHARP3;
    pub const D_SHARP4: Note = Note::D_SHARP4;
    pub const D_SHARP5: Note = Note::D_SHARP5;
    pub const D_SHARP6: Note = Note::D_SHARP6;
    pub const D_SHARP7: Note = Note::D_SHARP7;
    pub const D_SHARP8: Note = Note::D_SHARP8;
    pub const E_FLAT0: Note = Note::E_FLAT0;
    pub const E_FLAT1: Note = Note::E_FLAT1;
    pub const E_FLAT2: Note = Note::E_FLAT2;
    pub const E_FLAT3: Note = Note::E_FLAT3;
    pub const E_FLAT4: Note = Note::E_FLAT4;
    pub const E_FLAT5: Note = Note::E_FLAT5;
    pub const E_FLAT6: Note = Note::E_FLAT6;
    pub const E_FLAT7: Note = Note::E_FLAT7;
    pub const E_FLAT8: Note = Note::E_FLAT8;
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
    pub const F_SHARP0: Note = Note::F_SHARP0;
    pub const F_SHARP1: Note = Note::F_SHARP1;
    pub const F_SHARP2: Note = Note::F_SHARP2;
    pub const F_SHARP3: Note = Note::F_SHARP3;
    pub const F_SHARP4: Note = Note::F_SHARP4;
    pub const F_SHARP5: Note = Note::F_SHARP5;
    pub const F_SHARP6: Note = Note::F_SHARP6;
    pub const F_SHARP7: Note = Note::F_SHARP7;
    pub const F_SHARP8: Note = Note::F_SHARP8;
    pub const G_FLAT0: Note = Note::G_FLAT0;
    pub const G_FLAT1: Note = Note::G_FLAT1;
    pub const G_FLAT2: Note = Note::G_FLAT2;
    pub const G_FLAT3: Note = Note::G_FLAT3;
    pub const G_FLAT4: Note = Note::G_FLAT4;
    pub const G_FLAT5: Note = Note::G_FLAT5;
    pub const G_FLAT6: Note = Note::G_FLAT6;
    pub const G_FLAT7: Note = Note::G_FLAT7;
    pub const G_FLAT8: Note = Note::G_FLAT8;
    pub const G0: Note = Note::G0;
    pub const G1: Note = Note::G1;
    pub const G2: Note = Note::G2;
    pub const G3: Note = Note::G3;
    pub const G4: Note = Note::G4;
    pub const G5: Note = Note::G5;
    pub const G6: Note = Note::G6;
    pub const G7: Note = Note::G7;
    pub const G8: Note = Note::G8;
    pub const G_SHARP0: Note = Note::G_SHARP0;
    pub const G_SHARP1: Note = Note::G_SHARP1;
    pub const G_SHARP2: Note = Note::G_SHARP2;
    pub const G_SHARP3: Note = Note::G_SHARP3;
    pub const G_SHARP4: Note = Note::G_SHARP4;
    pub const G_SHARP5: Note = Note::G_SHARP5;
    pub const G_SHARP6: Note = Note::G_SHARP6;
    pub const G_SHARP7: Note = Note::G_SHARP7;
    pub const G_SHARP8: Note = Note::G_SHARP8;
    pub const A_FLAT0: Note = Note::A_FLAT0;
    pub const A_FLAT1: Note = Note::A_FLAT1;
    pub const A_FLAT2: Note = Note::A_FLAT2;
    pub const A_FLAT3: Note = Note::A_FLAT3;
    pub const A_FLAT4: Note = Note::A_FLAT4;
    pub const A_FLAT5: Note = Note::A_FLAT5;
    pub const A_FLAT6: Note = Note::A_FLAT6;
    pub const A_FLAT7: Note = Note::A_FLAT7;
    pub const A_FLAT8: Note = Note::A_FLAT8;
    pub const A0: Note = Note::A0;
    pub const A1: Note = Note::A1;
    pub const A2: Note = Note::A2;
    pub const A3: Note = Note::A3;
    pub const A4: Note = Note::A4;
    pub const A5: Note = Note::A5;
    pub const A6: Note = Note::A6;
    pub const A7: Note = Note::A7;
    pub const A8: Note = Note::A8;
    pub const A_SHARP0: Note = Note::A_SHARP0;
    pub const A_SHARP1: Note = Note::A_SHARP1;
    pub const A_SHARP2: Note = Note::A_SHARP2;
    pub const A_SHARP3: Note = Note::A_SHARP3;
    pub const A_SHARP4: Note = Note::A_SHARP4;
    pub const A_SHARP5: Note = Note::A_SHARP5;
    pub const A_SHARP6: Note = Note::A_SHARP6;
    pub const A_SHARP7: Note = Note::A_SHARP7;
    pub const A_SHARP8: Note = Note::A_SHARP8;
    pub const B_FLAT0: Note = Note::B_FLAT0;
    pub const B_FLAT1: Note = Note::B_FLAT1;
    pub const B_FLAT2: Note = Note::B_FLAT2;
    pub const B_FLAT3: Note = Note::B_FLAT3;
    pub const B_FLAT4: Note = Note::B_FLAT4;
    pub const B_FLAT5: Note = Note::B_FLAT5;
    pub const B_FLAT6: Note = Note::B_FLAT6;
    pub const B_FLAT7: Note = Note::B_FLAT7;
    pub const B_FLAT8: Note = Note::B_FLAT8;
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
