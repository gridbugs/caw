//! 12-tone equal temperament following the A_440Hz convention. Only allows representation of MIDI
//! notes. The frequency of A_4 (the A above middle C) is 440Hz. C_4 is considered to be middle C.
//! The lowest note is C in the octave "-1". The entire "-1" octave and most of octave 0 is below
//! the range of human hearing but might be useful for in-band custom control signals. The highest
//! note is G_9. Thus the 9th octave does not contain all notes. C is considered the first note in
//! each octave.
use caw_core::{Buf, ConstBuf, Sig, SigCtx, SigT};
use std::{fmt::Display, str::FromStr};

/// Octaves go from -1 to 8. Some notes in the 9th octave can be constructed, however the regular
/// `Note::new` function can't be passed the 9th octave since that would permit construction of
/// non-MIDI notes (those in the 9th octave above G_9).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Octave {
    /// To make the math easier octaves are represented by the index of the first note of the
    /// octave (c) divided by 12. Unfortunately this means that the first octave, named octave "-1"
    /// in MIDI parlance, has representation "0".
    c_midi_index_dividied_by_notes_per_octave: u8,
}

impl Octave {
    const MIN_OCTAVE: i8 = -1;
    const MAX_OCTAVE: i8 = 8;

    const fn from_index(i: i8) -> Self {
        assert!(i >= Self::MIN_OCTAVE && i <= Self::MAX_OCTAVE);
        Self {
            c_midi_index_dividied_by_notes_per_octave: (i + 1) as u8,
        }
    }

    const fn to_index(self) -> i8 {
        self.c_midi_index_dividied_by_notes_per_octave as i8 - 1
    }

    /// Returns the index of the C note in this octave.
    const fn c_midi_index(self) -> u8 {
        self.c_midi_index_dividied_by_notes_per_octave * NOTES_PER_OCTAVE
    }

    pub const _MINUS_1: Self = Self::from_index(-1);
    pub const _0: Self = Self::from_index(0);
    pub const _1: Self = Self::from_index(1);
    pub const _2: Self = Self::from_index(2);
    pub const _3: Self = Self::from_index(3);
    pub const _4: Self = Self::from_index(4);
    pub const _5: Self = Self::from_index(5);
    pub const _6: Self = Self::from_index(6);
    pub const _7: Self = Self::from_index(7);
    pub const _8: Self = Self::from_index(8);
}

/// Default to octave 4. This is fairly abitrary but needed to simplify code that derives
/// `Default` when it involves an `Octave`.
impl Default for Octave {
    fn default() -> Self {
        Octave::_4
    }
}

pub mod octave {
    use super::Octave;

    pub const _MINUS_1: Octave = Octave::_MINUS_1;
    pub const _0: Octave = Octave::_0;
    pub const _1: Octave = Octave::_1;
    pub const _2: Octave = Octave::_2;
    pub const _3: Octave = Octave::_3;
    pub const _4: Octave = Octave::_4;
    pub const _5: Octave = Octave::_5;
    pub const _6: Octave = Octave::_6;
    pub const _7: Octave = Octave::_7;
    pub const _8: Octave = Octave::_8;
}

pub const OCTAVE_MINUS_1: Octave = Octave::_MINUS_1;
pub const OCTAVE_0: Octave = Octave::_0;
pub const OCTAVE_1: Octave = Octave::_1;
pub const OCTAVE_2: Octave = Octave::_2;
pub const OCTAVE_3: Octave = Octave::_3;
pub const OCTAVE_4: Octave = Octave::_4;
pub const OCTAVE_5: Octave = Octave::_5;
pub const OCTAVE_6: Octave = Octave::_6;
pub const OCTAVE_7: Octave = Octave::_7;
pub const OCTAVE_8: Octave = Octave::_8;

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

    /// Returns a str representation of the note name where all accidentals are sharp, formatted
    /// like "C" or "C_sharp"
    pub const fn to_str_sharp(self) -> &'static str {
        match self.relative_midi_index {
            0 => "C",
            1 => "C_sharp",
            2 => "D",
            3 => "D_sharp",
            4 => "E",
            5 => "F",
            6 => "F_sharp",
            7 => "G",
            8 => "G_sharp",
            9 => "A",
            10 => "A_sharp",
            11 => "B",
            _ => unreachable!(),
        }
    }

    /// Parses a str like "C" or "C_sharp"
    pub fn from_str_sharp(s: &str) -> Option<Self> {
        let relative_midi_index = match s {
            "C" => 0,
            "C_sharp" => 1,
            "D" => 2,
            "D_sharp" => 3,
            "E" => 4,
            "F" => 5,
            "F_sharp" => 6,
            "G" => 7,
            "G_sharp" => 8,
            "A" => 9,
            "A_sharp" => 10,
            "B" => 11,
            _ => return None,
        };
        Some(Self {
            relative_midi_index,
        })
    }

    const fn wrapping_add_semitones(self, num_semitones: i8) -> Self {
        Self::from_index(
            (self.relative_midi_index as i8 + num_semitones)
                .rem_euclid(NOTES_PER_OCTAVE as i8) as u8,
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

const A_4_FREQ_HZ: f32 = 440.0;
const A_4_MIDI_INDEX: u8 = 69;
const C_9_MIDI_INDEX: u8 = 120;

pub fn freq_hz_of_midi_index(midi_index: u8) -> f32 {
    A_4_FREQ_HZ
        * (2_f32.powf(
            (midi_index as f32 - A_4_MIDI_INDEX as f32)
                / (NOTES_PER_OCTAVE as f32),
        ))
}

pub fn semitone_ratio(num_semitones: f32) -> f32 {
    2.0_f32.powf(num_semitones / (NOTES_PER_OCTAVE as f32))
}

pub const TONE_RATIO: f32 = 1.122_462;

/// Definition of notes based on MIDI tuned to A_440
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Note {
    midi_index: u8,
}

impl Note {
    pub const fn new(name: NoteName, octave: Octave) -> Self {
        Self {
            midi_index: octave.c_midi_index() + name.relative_midi_index,
        }
    }

    const fn new_octave_9(name: NoteName) -> Self {
        let midi_index = C_9_MIDI_INDEX + name.relative_midi_index;
        assert!(midi_index <= 127);
        Self { midi_index }
    }

    pub const fn to_midi_index(self) -> u8 {
        self.midi_index
    }

    pub fn freq_hz(self) -> f32 {
        freq_hz_of_midi_index(self.to_midi_index())
    }

    pub fn from_midi_index(midi_index: impl Into<u8>) -> Self {
        let midi_index = midi_index.into();
        assert!(midi_index <= 127);
        Self { midi_index }
    }

    pub const fn octave(self) -> Octave {
        Octave {
            c_midi_index_dividied_by_notes_per_octave: self.midi_index
                / NOTES_PER_OCTAVE,
        }
    }

    pub const fn note_name(self) -> NoteName {
        NoteName::from_index(self.midi_index % NOTES_PER_OCTAVE)
    }

    pub const fn add_semitones_checked(
        self,
        num_semitones: i16,
    ) -> Option<Self> {
        let midi_index = self.midi_index as i16 + num_semitones;
        if midi_index < 0 || midi_index > MAX_MIDI_INDEX as i16 {
            None
        } else {
            Some(Self {
                midi_index: midi_index as u8,
            })
        }
    }

    pub const fn add_octaves_checked(self, num_octaves: i8) -> Option<Self> {
        self.add_semitones_checked(num_octaves as i16 * NOTES_PER_OCTAVE as i16)
    }

    pub const fn add_semitones(self, num_semitones: i16) -> Self {
        Self {
            midi_index: (self.midi_index as i16 + num_semitones) as u8,
        }
    }

    pub const fn add_octaves(self, num_octaves: i8) -> Self {
        self.add_semitones(num_octaves as i16 * NOTES_PER_OCTAVE as i16)
    }
}

/// Example formats: "C_sharp:4", "C:4". Notes in octave "-1" are written like "C_sharp:-1" or
/// "C:-1".
impl Display for Note {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}",
            self.note_name().to_str_sharp(),
            self.octave().to_index()
        )
    }
}

/// Expected format: "C_sharp-4", "C-4"
impl FromStr for Note {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(":");
        if let Some(name) = split.next() {
            if let Some(name) = NoteName::from_str_sharp(name) {
                if let Some(octave_index) = split.next() {
                    match octave_index.parse::<i8>() {
                        Ok(octave_index) => {
                            if octave_index <= Octave::MAX_OCTAVE {
                                if split.next().is_none() {
                                    Ok(Note::new(
                                        name,
                                        Octave::from_index(octave_index),
                                    ))
                                } else {
                                    Err(format!(
                                        "Multiple colons in note string."
                                    ))
                                }
                            } else {
                                Err(format!(
                                    "Octave index {} too high (max is {}).",
                                    octave_index,
                                    Octave::MAX_OCTAVE
                                ))
                            }
                        }
                        Err(e) => {
                            Err(format!("Failed to parse octave index: {}", e))
                        }
                    }
                } else {
                    Err(format!("No colons in note string."))
                }
            } else {
                Err(format!("Failed to parse note name: {}", name))
            }
        } else {
            Err(format!("Failed to parse note name."))
        }
    }
}

pub trait IntoNoteFreqHz<N>
where
    N: SigT<Item = Note>,
{
    fn freq_hz(self) -> Sig<impl SigT<Item = f32>>;
}

impl<N> IntoNoteFreqHz<N> for Sig<N>
where
    N: SigT<Item = Note>,
{
    fn freq_hz(self) -> Sig<impl SigT<Item = f32>> {
        self.map(|note| note.freq_hz())
    }
}

impl SigT for Note {
    type Item = Self;

    fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
        ConstBuf {
            value: *self,
            count: ctx.num_samples,
        }
    }
}

/// Arbitrary default value to help when a temporary value must be set before updating a note by
/// some other process, as signals cannot have undefined values. The default note is C_4.
impl Default for Note {
    fn default() -> Self {
        note::C_4
    }
}

pub mod chord {
    use super::{Note, NoteName, Octave, note_name};
    use caw_core::{Buf, ConstBuf, SigCtx, SigT};
    use smallvec::{SmallVec, smallvec};

    pub struct Notes(SmallVec<[Note; 4]>);

    impl Notes {
        fn new() -> Self {
            Self(smallvec![])
        }
        pub fn iter(&self) -> impl Iterator<Item = &Note> {
            self.0.iter()
        }
        fn push(&mut self, note: Note) {
            self.0.push(note);
        }
    }

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

        pub fn with_semitones_above_root<F: FnMut(i8, ChordPosition)>(
            &self,
            mut f: F,
        ) {
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

    fn wrap_note_within_octave(
        octave_base: Note,
        root: NoteName,
        semitones_above: i8,
    ) -> Note {
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
                    .relative_midi_index,
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
                root_octave: Octave::_4,
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
        pub const fn new(root: NoteName, typ: ChordType) -> Self {
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

        fn with_notes_in_octave<F: FnMut(Note)>(
            self,
            octave_base: Note,
            mut f: F,
        ) {
            self.typ.with_semitones_above_root(|semitones_above, _| {
                let note = wrap_note_within_octave(
                    octave_base,
                    self.root,
                    semitones_above,
                )
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
            self.typ.with_semitones_above_root(
                |semitones_above, chord_position| {
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
                },
            );
        }

        pub fn with_notes<F: FnMut(Note)>(self, inversion: Inversion, f: F) {
            match inversion {
                Inversion::WithRootOctave {
                    root_octave,
                    lowest_position,
                } => {
                    self.with_notes_root_octave(root_octave, lowest_position, f)
                }
                Inversion::InOctave { octave_base } => {
                    self.with_notes_in_octave(octave_base, f)
                }
            }
        }

        pub fn notes(self, inversion: Inversion) -> Notes {
            let mut ret = Notes::new();
            self.with_notes(inversion, |note| {
                ret.push(note);
            });
            ret
        }
    }

    pub const fn chord(root: NoteName, typ: ChordType) -> Chord {
        Chord::new(root, typ)
    }

    impl SigT for Chord {
        type Item = Self;

        fn sample(&mut self, ctx: &SigCtx) -> impl Buf<Self::Item> {
            ConstBuf {
                value: *self,
                count: ctx.num_samples,
            }
        }
    }

    /// Arbitrary default value to help when a temporary value must be set before updating a note by
    /// some other process, as signals cannot have undefined values. The default chord is C major.
    impl Default for Chord {
        fn default() -> Self {
            chord(note_name::C, MAJOR)
        }
    }
}

impl Note {
    pub const C_MINUS_1: Self = Self::new(NoteName::C, OCTAVE_MINUS_1);
    pub const C_0: Self = Self::new(NoteName::C, OCTAVE_0);
    pub const C_1: Self = Self::new(NoteName::C, OCTAVE_1);
    pub const C_2: Self = Self::new(NoteName::C, OCTAVE_2);
    pub const C_3: Self = Self::new(NoteName::C, OCTAVE_3);
    pub const C_4: Self = Self::new(NoteName::C, OCTAVE_4);
    pub const C_5: Self = Self::new(NoteName::C, OCTAVE_5);
    pub const C_6: Self = Self::new(NoteName::C, OCTAVE_6);
    pub const C_7: Self = Self::new(NoteName::C, OCTAVE_7);
    pub const C_8: Self = Self::new(NoteName::C, OCTAVE_8);
    pub const C_9: Self = Self::new_octave_9(NoteName::C);
    pub const C_SHARP_MINUS_1: Self =
        Self::new(NoteName::C_SHARP, OCTAVE_MINUS_1);
    pub const C_SHARP_0: Self = Self::new(NoteName::C_SHARP, OCTAVE_0);
    pub const C_SHARP_1: Self = Self::new(NoteName::C_SHARP, OCTAVE_1);
    pub const C_SHARP_2: Self = Self::new(NoteName::C_SHARP, OCTAVE_2);
    pub const C_SHARP_3: Self = Self::new(NoteName::C_SHARP, OCTAVE_3);
    pub const C_SHARP_4: Self = Self::new(NoteName::C_SHARP, OCTAVE_4);
    pub const C_SHARP_5: Self = Self::new(NoteName::C_SHARP, OCTAVE_5);
    pub const C_SHARP_6: Self = Self::new(NoteName::C_SHARP, OCTAVE_6);
    pub const C_SHARP_7: Self = Self::new(NoteName::C_SHARP, OCTAVE_7);
    pub const C_SHARP_8: Self = Self::new(NoteName::C_SHARP, OCTAVE_8);
    pub const C_SHARP_9: Self = Self::new_octave_9(NoteName::C_SHARP);
    pub const D_FLAT_MINUS_1: Self =
        Self::new(NoteName::D_FLAT, OCTAVE_MINUS_1);
    pub const D_FLAT_0: Self = Self::new(NoteName::D_FLAT, OCTAVE_0);
    pub const D_FLAT_1: Self = Self::new(NoteName::D_FLAT, OCTAVE_1);
    pub const D_FLAT_2: Self = Self::new(NoteName::D_FLAT, OCTAVE_2);
    pub const D_FLAT_3: Self = Self::new(NoteName::D_FLAT, OCTAVE_3);
    pub const D_FLAT_4: Self = Self::new(NoteName::D_FLAT, OCTAVE_4);
    pub const D_FLAT_5: Self = Self::new(NoteName::D_FLAT, OCTAVE_5);
    pub const D_FLAT_6: Self = Self::new(NoteName::D_FLAT, OCTAVE_6);
    pub const D_FLAT_7: Self = Self::new(NoteName::D_FLAT, OCTAVE_7);
    pub const D_FLAT_8: Self = Self::new(NoteName::D_FLAT, OCTAVE_8);
    pub const D_FLAT_9: Self = Self::new_octave_9(NoteName::D_FLAT);
    pub const D_MINUS_1: Self = Self::new(NoteName::D, OCTAVE_MINUS_1);
    pub const D_0: Self = Self::new(NoteName::D, OCTAVE_0);
    pub const D_1: Self = Self::new(NoteName::D, OCTAVE_1);
    pub const D_2: Self = Self::new(NoteName::D, OCTAVE_2);
    pub const D_3: Self = Self::new(NoteName::D, OCTAVE_3);
    pub const D_4: Self = Self::new(NoteName::D, OCTAVE_4);
    pub const D_5: Self = Self::new(NoteName::D, OCTAVE_5);
    pub const D_6: Self = Self::new(NoteName::D, OCTAVE_6);
    pub const D_7: Self = Self::new(NoteName::D, OCTAVE_7);
    pub const D_8: Self = Self::new(NoteName::D, OCTAVE_8);
    pub const D_9: Self = Self::new_octave_9(NoteName::D);
    pub const D_SHARP_MINUS_1: Self =
        Self::new(NoteName::D_SHARP, OCTAVE_MINUS_1);
    pub const D_SHARP_0: Self = Self::new(NoteName::D_SHARP, OCTAVE_0);
    pub const D_SHARP_1: Self = Self::new(NoteName::D_SHARP, OCTAVE_1);
    pub const D_SHARP_2: Self = Self::new(NoteName::D_SHARP, OCTAVE_2);
    pub const D_SHARP_3: Self = Self::new(NoteName::D_SHARP, OCTAVE_3);
    pub const D_SHARP_4: Self = Self::new(NoteName::D_SHARP, OCTAVE_4);
    pub const D_SHARP_5: Self = Self::new(NoteName::D_SHARP, OCTAVE_5);
    pub const D_SHARP_6: Self = Self::new(NoteName::D_SHARP, OCTAVE_6);
    pub const D_SHARP_7: Self = Self::new(NoteName::D_SHARP, OCTAVE_7);
    pub const D_SHARP_8: Self = Self::new(NoteName::D_SHARP, OCTAVE_8);
    pub const D_SHARP_9: Self = Self::new_octave_9(NoteName::D_SHARP);
    pub const E_FLAT_MINUS_1: Self =
        Self::new(NoteName::E_FLAT, OCTAVE_MINUS_1);
    pub const E_FLAT_0: Self = Self::new(NoteName::E_FLAT, OCTAVE_0);
    pub const E_FLAT_1: Self = Self::new(NoteName::E_FLAT, OCTAVE_1);
    pub const E_FLAT_2: Self = Self::new(NoteName::E_FLAT, OCTAVE_2);
    pub const E_FLAT_3: Self = Self::new(NoteName::E_FLAT, OCTAVE_3);
    pub const E_FLAT_4: Self = Self::new(NoteName::E_FLAT, OCTAVE_4);
    pub const E_FLAT_5: Self = Self::new(NoteName::E_FLAT, OCTAVE_5);
    pub const E_FLAT_6: Self = Self::new(NoteName::E_FLAT, OCTAVE_6);
    pub const E_FLAT_7: Self = Self::new(NoteName::E_FLAT, OCTAVE_7);
    pub const E_FLAT_8: Self = Self::new(NoteName::E_FLAT, OCTAVE_8);
    pub const E_FLAT_9: Self = Self::new_octave_9(NoteName::E_FLAT);
    pub const E_MINUS_1: Self = Self::new(NoteName::E, OCTAVE_MINUS_1);
    pub const E_0: Self = Self::new(NoteName::E, OCTAVE_0);
    pub const E_1: Self = Self::new(NoteName::E, OCTAVE_1);
    pub const E_2: Self = Self::new(NoteName::E, OCTAVE_2);
    pub const E_3: Self = Self::new(NoteName::E, OCTAVE_3);
    pub const E_4: Self = Self::new(NoteName::E, OCTAVE_4);
    pub const E_5: Self = Self::new(NoteName::E, OCTAVE_5);
    pub const E_6: Self = Self::new(NoteName::E, OCTAVE_6);
    pub const E_7: Self = Self::new(NoteName::E, OCTAVE_7);
    pub const E_8: Self = Self::new(NoteName::E, OCTAVE_8);
    pub const E_9: Self = Self::new_octave_9(NoteName::E);
    pub const F_MINUS_1: Self = Self::new(NoteName::F, OCTAVE_MINUS_1);
    pub const F_0: Self = Self::new(NoteName::F, OCTAVE_0);
    pub const F_1: Self = Self::new(NoteName::F, OCTAVE_1);
    pub const F_2: Self = Self::new(NoteName::F, OCTAVE_2);
    pub const F_3: Self = Self::new(NoteName::F, OCTAVE_3);
    pub const F_4: Self = Self::new(NoteName::F, OCTAVE_4);
    pub const F_5: Self = Self::new(NoteName::F, OCTAVE_5);
    pub const F_6: Self = Self::new(NoteName::F, OCTAVE_6);
    pub const F_7: Self = Self::new(NoteName::F, OCTAVE_7);
    pub const F_8: Self = Self::new(NoteName::F, OCTAVE_8);
    pub const F_9: Self = Self::new_octave_9(NoteName::F);
    pub const F_SHARP_MINUS_1: Self =
        Self::new(NoteName::F_SHARP, OCTAVE_MINUS_1);
    pub const F_SHARP_0: Self = Self::new(NoteName::F_SHARP, OCTAVE_0);
    pub const F_SHARP_1: Self = Self::new(NoteName::F_SHARP, OCTAVE_1);
    pub const F_SHARP_2: Self = Self::new(NoteName::F_SHARP, OCTAVE_2);
    pub const F_SHARP_3: Self = Self::new(NoteName::F_SHARP, OCTAVE_3);
    pub const F_SHARP_4: Self = Self::new(NoteName::F_SHARP, OCTAVE_4);
    pub const F_SHARP_5: Self = Self::new(NoteName::F_SHARP, OCTAVE_5);
    pub const F_SHARP_6: Self = Self::new(NoteName::F_SHARP, OCTAVE_6);
    pub const F_SHARP_7: Self = Self::new(NoteName::F_SHARP, OCTAVE_7);
    pub const F_SHARP_8: Self = Self::new(NoteName::F_SHARP, OCTAVE_8);
    pub const F_SHARP_9: Self = Self::new_octave_9(NoteName::F_SHARP);
    pub const G_FLAT_MINUS_1: Self =
        Self::new(NoteName::G_FLAT, OCTAVE_MINUS_1);
    pub const G_FLAT_0: Self = Self::new(NoteName::G_FLAT, OCTAVE_0);
    pub const G_FLAT_1: Self = Self::new(NoteName::G_FLAT, OCTAVE_1);
    pub const G_FLAT_2: Self = Self::new(NoteName::G_FLAT, OCTAVE_2);
    pub const G_FLAT_3: Self = Self::new(NoteName::G_FLAT, OCTAVE_3);
    pub const G_FLAT_4: Self = Self::new(NoteName::G_FLAT, OCTAVE_4);
    pub const G_FLAT_5: Self = Self::new(NoteName::G_FLAT, OCTAVE_5);
    pub const G_FLAT_6: Self = Self::new(NoteName::G_FLAT, OCTAVE_6);
    pub const G_FLAT_7: Self = Self::new(NoteName::G_FLAT, OCTAVE_7);
    pub const G_FLAT_8: Self = Self::new(NoteName::G_FLAT, OCTAVE_8);
    pub const G_FLAT_9: Self = Self::new_octave_9(NoteName::G_FLAT);
    pub const G_MINUS_1: Self = Self::new(NoteName::G, OCTAVE_MINUS_1);
    pub const G_0: Self = Self::new(NoteName::G, OCTAVE_0);
    pub const G_1: Self = Self::new(NoteName::G, OCTAVE_1);
    pub const G_2: Self = Self::new(NoteName::G, OCTAVE_2);
    pub const G_3: Self = Self::new(NoteName::G, OCTAVE_3);
    pub const G_4: Self = Self::new(NoteName::G, OCTAVE_4);
    pub const G_5: Self = Self::new(NoteName::G, OCTAVE_5);
    pub const G_6: Self = Self::new(NoteName::G, OCTAVE_6);
    pub const G_7: Self = Self::new(NoteName::G, OCTAVE_7);
    pub const G_8: Self = Self::new(NoteName::G, OCTAVE_8);
    pub const G_9: Self = Self::new_octave_9(NoteName::G);
    pub const G_SHARP_MINUS_1: Self =
        Self::new(NoteName::G_SHARP, OCTAVE_MINUS_1);
    pub const G_SHARP_0: Self = Self::new(NoteName::G_SHARP, OCTAVE_0);
    pub const G_SHARP_1: Self = Self::new(NoteName::G_SHARP, OCTAVE_1);
    pub const G_SHARP_2: Self = Self::new(NoteName::G_SHARP, OCTAVE_2);
    pub const G_SHARP_3: Self = Self::new(NoteName::G_SHARP, OCTAVE_3);
    pub const G_SHARP_4: Self = Self::new(NoteName::G_SHARP, OCTAVE_4);
    pub const G_SHARP_5: Self = Self::new(NoteName::G_SHARP, OCTAVE_5);
    pub const G_SHARP_6: Self = Self::new(NoteName::G_SHARP, OCTAVE_6);
    pub const G_SHARP_7: Self = Self::new(NoteName::G_SHARP, OCTAVE_7);
    pub const G_SHARP_8: Self = Self::new(NoteName::G_SHARP, OCTAVE_8);
    pub const G_SHARP_9: Self = Self::new_octave_9(NoteName::G_SHARP);
    pub const A_FLAT_MINUS_1: Self =
        Self::new(NoteName::A_FLAT, OCTAVE_MINUS_1);
    pub const A_FLAT_0: Self = Self::new(NoteName::A_FLAT, OCTAVE_0);
    pub const A_FLAT_1: Self = Self::new(NoteName::A_FLAT, OCTAVE_1);
    pub const A_FLAT_2: Self = Self::new(NoteName::A_FLAT, OCTAVE_2);
    pub const A_FLAT_3: Self = Self::new(NoteName::A_FLAT, OCTAVE_3);
    pub const A_FLAT_4: Self = Self::new(NoteName::A_FLAT, OCTAVE_4);
    pub const A_FLAT_5: Self = Self::new(NoteName::A_FLAT, OCTAVE_5);
    pub const A_FLAT_6: Self = Self::new(NoteName::A_FLAT, OCTAVE_6);
    pub const A_FLAT_7: Self = Self::new(NoteName::A_FLAT, OCTAVE_7);
    pub const A_FLAT_8: Self = Self::new(NoteName::A_FLAT, OCTAVE_8);
    pub const A_MINUS_1: Self = Self::new(NoteName::A, OCTAVE_MINUS_1);
    pub const A_0: Self = Self::new(NoteName::A, OCTAVE_0);
    pub const A_1: Self = Self::new(NoteName::A, OCTAVE_1);
    pub const A_2: Self = Self::new(NoteName::A, OCTAVE_2);
    pub const A_3: Self = Self::new(NoteName::A, OCTAVE_3);
    pub const A_4: Self = Self::new(NoteName::A, OCTAVE_4);
    pub const A_5: Self = Self::new(NoteName::A, OCTAVE_5);
    pub const A_6: Self = Self::new(NoteName::A, OCTAVE_6);
    pub const A_7: Self = Self::new(NoteName::A, OCTAVE_7);
    pub const A_8: Self = Self::new(NoteName::A, OCTAVE_8);
    pub const A_SHARP_MINUS_1: Self =
        Self::new(NoteName::A_SHARP, OCTAVE_MINUS_1);
    pub const A_SHARP_0: Self = Self::new(NoteName::A_SHARP, OCTAVE_0);
    pub const A_SHARP_1: Self = Self::new(NoteName::A_SHARP, OCTAVE_1);
    pub const A_SHARP_2: Self = Self::new(NoteName::A_SHARP, OCTAVE_2);
    pub const A_SHARP_3: Self = Self::new(NoteName::A_SHARP, OCTAVE_3);
    pub const A_SHARP_4: Self = Self::new(NoteName::A_SHARP, OCTAVE_4);
    pub const A_SHARP_5: Self = Self::new(NoteName::A_SHARP, OCTAVE_5);
    pub const A_SHARP_6: Self = Self::new(NoteName::A_SHARP, OCTAVE_6);
    pub const A_SHARP_7: Self = Self::new(NoteName::A_SHARP, OCTAVE_7);
    pub const A_SHARP_8: Self = Self::new(NoteName::A_SHARP, OCTAVE_8);
    pub const B_FLAT_MINUS_1: Self =
        Self::new(NoteName::B_FLAT, OCTAVE_MINUS_1);
    pub const B_FLAT_0: Self = Self::new(NoteName::B_FLAT, OCTAVE_0);
    pub const B_FLAT_1: Self = Self::new(NoteName::B_FLAT, OCTAVE_1);
    pub const B_FLAT_2: Self = Self::new(NoteName::B_FLAT, OCTAVE_2);
    pub const B_FLAT_3: Self = Self::new(NoteName::B_FLAT, OCTAVE_3);
    pub const B_FLAT_4: Self = Self::new(NoteName::B_FLAT, OCTAVE_4);
    pub const B_FLAT_5: Self = Self::new(NoteName::B_FLAT, OCTAVE_5);
    pub const B_FLAT_6: Self = Self::new(NoteName::B_FLAT, OCTAVE_6);
    pub const B_FLAT_7: Self = Self::new(NoteName::B_FLAT, OCTAVE_7);
    pub const B_FLAT_8: Self = Self::new(NoteName::B_FLAT, OCTAVE_8);
    pub const B_MINUS_1: Self = Self::new(NoteName::B, OCTAVE_MINUS_1);
    pub const B_0: Self = Self::new(NoteName::B, OCTAVE_0);
    pub const B_1: Self = Self::new(NoteName::B, OCTAVE_1);
    pub const B_2: Self = Self::new(NoteName::B, OCTAVE_2);
    pub const B_3: Self = Self::new(NoteName::B, OCTAVE_3);
    pub const B_4: Self = Self::new(NoteName::B, OCTAVE_4);
    pub const B_5: Self = Self::new(NoteName::B, OCTAVE_5);
    pub const B_6: Self = Self::new(NoteName::B, OCTAVE_6);
    pub const B_7: Self = Self::new(NoteName::B, OCTAVE_7);
    pub const B_8: Self = Self::new(NoteName::B, OCTAVE_8);
}

/// Duplicated from `Note` so it's possible to bring all notes into scope by using this module.
pub mod note {
    pub use super::Note;
    pub const C_MINUS_1: Note = Note::C_MINUS_1;
    pub const C_0: Note = Note::C_0;
    pub const C_1: Note = Note::C_1;
    pub const C_2: Note = Note::C_2;
    pub const C_3: Note = Note::C_3;
    pub const C_4: Note = Note::C_4;
    pub const C_5: Note = Note::C_5;
    pub const C_6: Note = Note::C_6;
    pub const C_7: Note = Note::C_7;
    pub const C_8: Note = Note::C_8;
    pub const C_9: Note = Note::C_9;
    pub const C_SHARP_MINUS_1: Note = Note::C_SHARP_MINUS_1;
    pub const C_SHARP_0: Note = Note::C_SHARP_0;
    pub const C_SHARP_1: Note = Note::C_SHARP_1;
    pub const C_SHARP_2: Note = Note::C_SHARP_2;
    pub const C_SHARP_3: Note = Note::C_SHARP_3;
    pub const C_SHARP_4: Note = Note::C_SHARP_4;
    pub const C_SHARP_5: Note = Note::C_SHARP_5;
    pub const C_SHARP_6: Note = Note::C_SHARP_6;
    pub const C_SHARP_7: Note = Note::C_SHARP_7;
    pub const C_SHARP_8: Note = Note::C_SHARP_8;
    pub const C_SHARP_9: Note = Note::C_SHARP_9;
    pub const D_FLAT_MINUS_1: Note = Note::D_FLAT_MINUS_1;
    pub const D_FLAT_0: Note = Note::D_FLAT_0;
    pub const D_FLAT_1: Note = Note::D_FLAT_1;
    pub const D_FLAT_2: Note = Note::D_FLAT_2;
    pub const D_FLAT_3: Note = Note::D_FLAT_3;
    pub const D_FLAT_4: Note = Note::D_FLAT_4;
    pub const D_FLAT_5: Note = Note::D_FLAT_5;
    pub const D_FLAT_6: Note = Note::D_FLAT_6;
    pub const D_FLAT_7: Note = Note::D_FLAT_7;
    pub const D_FLAT_8: Note = Note::D_FLAT_8;
    pub const D_FLAT_9: Note = Note::D_FLAT_9;
    pub const D_MINUS_1: Note = Note::D_MINUS_1;
    pub const D_0: Note = Note::D_0;
    pub const D_1: Note = Note::D_1;
    pub const D_2: Note = Note::D_2;
    pub const D_3: Note = Note::D_3;
    pub const D_4: Note = Note::D_4;
    pub const D_5: Note = Note::D_5;
    pub const D_6: Note = Note::D_6;
    pub const D_7: Note = Note::D_7;
    pub const D_8: Note = Note::D_8;
    pub const D_9: Note = Note::D_9;
    pub const D_SHARP_MINUS_1: Note = Note::D_SHARP_MINUS_1;
    pub const D_SHARP_0: Note = Note::D_SHARP_0;
    pub const D_SHARP_1: Note = Note::D_SHARP_1;
    pub const D_SHARP_2: Note = Note::D_SHARP_2;
    pub const D_SHARP_3: Note = Note::D_SHARP_3;
    pub const D_SHARP_4: Note = Note::D_SHARP_4;
    pub const D_SHARP_5: Note = Note::D_SHARP_5;
    pub const D_SHARP_6: Note = Note::D_SHARP_6;
    pub const D_SHARP_7: Note = Note::D_SHARP_7;
    pub const D_SHARP_8: Note = Note::D_SHARP_8;
    pub const D_SHARP_9: Note = Note::D_SHARP_9;
    pub const E_FLAT_MINUS_1: Note = Note::E_FLAT_MINUS_1;
    pub const E_FLAT_0: Note = Note::E_FLAT_0;
    pub const E_FLAT_1: Note = Note::E_FLAT_1;
    pub const E_FLAT_2: Note = Note::E_FLAT_2;
    pub const E_FLAT_3: Note = Note::E_FLAT_3;
    pub const E_FLAT_4: Note = Note::E_FLAT_4;
    pub const E_FLAT_5: Note = Note::E_FLAT_5;
    pub const E_FLAT_6: Note = Note::E_FLAT_6;
    pub const E_FLAT_7: Note = Note::E_FLAT_7;
    pub const E_FLAT_8: Note = Note::E_FLAT_8;
    pub const E_FLAT_9: Note = Note::E_FLAT_9;
    pub const E_MINUS_1: Note = Note::E_MINUS_1;
    pub const E_0: Note = Note::E_0;
    pub const E_1: Note = Note::E_1;
    pub const E_2: Note = Note::E_2;
    pub const E_3: Note = Note::E_3;
    pub const E_4: Note = Note::E_4;
    pub const E_5: Note = Note::E_5;
    pub const E_6: Note = Note::E_6;
    pub const E_7: Note = Note::E_7;
    pub const E_8: Note = Note::E_8;
    pub const E_9: Note = Note::E_9;
    pub const F_MINUS_1: Note = Note::F_MINUS_1;
    pub const F_0: Note = Note::F_0;
    pub const F_1: Note = Note::F_1;
    pub const F_2: Note = Note::F_2;
    pub const F_3: Note = Note::F_3;
    pub const F_4: Note = Note::F_4;
    pub const F_5: Note = Note::F_5;
    pub const F_6: Note = Note::F_6;
    pub const F_7: Note = Note::F_7;
    pub const F_8: Note = Note::F_8;
    pub const F_9: Note = Note::F_9;
    pub const F_SHARP_MINUS_1: Note = Note::F_SHARP_MINUS_1;
    pub const F_SHARP_0: Note = Note::F_SHARP_0;
    pub const F_SHARP_1: Note = Note::F_SHARP_1;
    pub const F_SHARP_2: Note = Note::F_SHARP_2;
    pub const F_SHARP_3: Note = Note::F_SHARP_3;
    pub const F_SHARP_4: Note = Note::F_SHARP_4;
    pub const F_SHARP_5: Note = Note::F_SHARP_5;
    pub const F_SHARP_6: Note = Note::F_SHARP_6;
    pub const F_SHARP_7: Note = Note::F_SHARP_7;
    pub const F_SHARP_8: Note = Note::F_SHARP_8;
    pub const F_SHARP_9: Note = Note::F_SHARP_9;
    pub const G_FLAT_MINUS_1: Note = Note::G_FLAT_MINUS_1;
    pub const G_FLAT_0: Note = Note::G_FLAT_0;
    pub const G_FLAT_1: Note = Note::G_FLAT_1;
    pub const G_FLAT_2: Note = Note::G_FLAT_2;
    pub const G_FLAT_3: Note = Note::G_FLAT_3;
    pub const G_FLAT_4: Note = Note::G_FLAT_4;
    pub const G_FLAT_5: Note = Note::G_FLAT_5;
    pub const G_FLAT_6: Note = Note::G_FLAT_6;
    pub const G_FLAT_7: Note = Note::G_FLAT_7;
    pub const G_FLAT_8: Note = Note::G_FLAT_8;
    pub const G_FLAT_9: Note = Note::G_FLAT_9;
    pub const G_MINUS_1: Note = Note::G_MINUS_1;
    pub const G_0: Note = Note::G_0;
    pub const G_1: Note = Note::G_1;
    pub const G_2: Note = Note::G_2;
    pub const G_3: Note = Note::G_3;
    pub const G_4: Note = Note::G_4;
    pub const G_5: Note = Note::G_5;
    pub const G_6: Note = Note::G_6;
    pub const G_7: Note = Note::G_7;
    pub const G_8: Note = Note::G_8;
    pub const G_9: Note = Note::G_9;
    pub const G_SHARP_MINUS_1: Note = Note::G_SHARP_MINUS_1;
    pub const G_SHARP_0: Note = Note::G_SHARP_0;
    pub const G_SHARP_1: Note = Note::G_SHARP_1;
    pub const G_SHARP_2: Note = Note::G_SHARP_2;
    pub const G_SHARP_3: Note = Note::G_SHARP_3;
    pub const G_SHARP_4: Note = Note::G_SHARP_4;
    pub const G_SHARP_5: Note = Note::G_SHARP_5;
    pub const G_SHARP_6: Note = Note::G_SHARP_6;
    pub const G_SHARP_7: Note = Note::G_SHARP_7;
    pub const G_SHARP_8: Note = Note::G_SHARP_8;
    pub const A_FLAT_MINUS_1: Note = Note::A_FLAT_MINUS_1;
    pub const A_FLAT_0: Note = Note::A_FLAT_0;
    pub const A_FLAT_1: Note = Note::A_FLAT_1;
    pub const A_FLAT_2: Note = Note::A_FLAT_2;
    pub const A_FLAT_3: Note = Note::A_FLAT_3;
    pub const A_FLAT_4: Note = Note::A_FLAT_4;
    pub const A_FLAT_5: Note = Note::A_FLAT_5;
    pub const A_FLAT_6: Note = Note::A_FLAT_6;
    pub const A_FLAT_7: Note = Note::A_FLAT_7;
    pub const A_FLAT_8: Note = Note::A_FLAT_8;
    pub const A_MINUS_1: Note = Note::A_MINUS_1;
    pub const A_0: Note = Note::A_0;
    pub const A_1: Note = Note::A_1;
    pub const A_2: Note = Note::A_2;
    pub const A_3: Note = Note::A_3;
    pub const A_4: Note = Note::A_4;
    pub const A_5: Note = Note::A_5;
    pub const A_6: Note = Note::A_6;
    pub const A_7: Note = Note::A_7;
    pub const A_8: Note = Note::A_8;
    pub const A_SHARP_MINUS_1: Note = Note::A_SHARP_MINUS_1;
    pub const A_SHARP_0: Note = Note::A_SHARP_0;
    pub const A_SHARP_1: Note = Note::A_SHARP_1;
    pub const A_SHARP_2: Note = Note::A_SHARP_2;
    pub const A_SHARP_3: Note = Note::A_SHARP_3;
    pub const A_SHARP_4: Note = Note::A_SHARP_4;
    pub const A_SHARP_5: Note = Note::A_SHARP_5;
    pub const A_SHARP_6: Note = Note::A_SHARP_6;
    pub const A_SHARP_7: Note = Note::A_SHARP_7;
    pub const A_SHARP_8: Note = Note::A_SHARP_8;
    pub const B_FLAT_MINUS_1: Note = Note::B_FLAT_MINUS_1;
    pub const B_FLAT_0: Note = Note::B_FLAT_0;
    pub const B_FLAT_1: Note = Note::B_FLAT_1;
    pub const B_FLAT_2: Note = Note::B_FLAT_2;
    pub const B_FLAT_3: Note = Note::B_FLAT_3;
    pub const B_FLAT_4: Note = Note::B_FLAT_4;
    pub const B_FLAT_5: Note = Note::B_FLAT_5;
    pub const B_FLAT_6: Note = Note::B_FLAT_6;
    pub const B_FLAT_7: Note = Note::B_FLAT_7;
    pub const B_FLAT_8: Note = Note::B_FLAT_8;
    pub const B_MINUS_1: Note = Note::B_MINUS_1;
    pub const B_0: Note = Note::B_0;
    pub const B_1: Note = Note::B_1;
    pub const B_2: Note = Note::B_2;
    pub const B_3: Note = Note::B_3;
    pub const B_4: Note = Note::B_4;
    pub const B_5: Note = Note::B_5;
    pub const B_6: Note = Note::B_6;
    pub const B_7: Note = Note::B_7;
    pub const B_8: Note = Note::B_8;
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn octave_round_trip() {
        assert_eq!(Note::new(note_name::C, OCTAVE_0).octave(), OCTAVE_0);
    }

    #[test]
    fn note_name_round_trip() {
        assert_eq!(Note::new(note_name::D, OCTAVE_3).note_name(), note_name::D);
    }

    #[test]
    fn string_round_trip() {
        assert_eq!(note::D_6.to_string().parse::<Note>().unwrap(), note::D_6);
        assert_eq!(
            note::A_SHARP_5.to_string().parse::<Note>().unwrap(),
            note::A_SHARP_5
        );
    }

    #[test]
    fn min_note_midi_index() {
        assert_eq!(Note::C_MINUS_1.to_midi_index(), 0);
    }

    #[test]
    fn max_note_midi_index() {
        assert_eq!(Note::G_9.to_midi_index(), 127);
    }
}
