use caw_core::*;
pub use caw_midi::{MidiMessages, MidiMessagesT};
use midly::{
    num::u4, Format, MetaMessage, MidiMessage, Smf, Timing, TrackEvent,
    TrackEventKind,
};
use std::{fs, path::Path};

#[derive(Debug, Clone, Copy)]
pub struct MidiEvent {
    pub channel: u4,
    pub message: MidiMessage,
}

pub struct MidiFile {
    smf: Smf<'static>,
}

impl MidiFile {
    pub fn read(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let raw = fs::read(path)?;
        let smf = Smf::parse(&raw)?.make_static();
        Ok(Self { smf })
    }

    pub fn format(&self) -> Format {
        self.smf.header.format
    }

    pub fn num_tracks(&self) -> usize {
        self.smf.tracks.len()
    }

    pub fn track(
        &self,
        track_index: usize,
        default_s_per_beat: f32,
    ) -> anyhow::Result<TrackState> {
        if let Some(events) = self.smf.tracks.get(track_index) {
            let track_state = TrackState::new(
                events,
                self.smf.header.timing,
                default_s_per_beat,
            );
            Ok(track_state)
        } else {
            anyhow::bail!(
                "Track index {} is out of range (there are {} tracks)",
                track_index,
                self.num_tracks()
            )
        }
    }
}

pub struct TrackState {
    track: Vec<TrackEvent<'static>>,
    timing: Timing,
    tick_duration_s: f32,
    next_index: usize,
    next_tick: u32,
    current_time_s: f32,
    next_tick_time_s: f32,
}

impl TrackState {
    pub fn new(
        track: &[TrackEvent<'_>],
        timing: Timing,
        default_s_per_beat: f32,
    ) -> Self {
        let track = track
            .iter()
            .map(|event| event.to_static())
            .collect::<Vec<_>>();
        let tick_duration_s = match timing {
            Timing::Metrical(ticks_per_beat) => {
                default_s_per_beat / (ticks_per_beat.as_int() as f32)
            }
            Timing::Timecode(frames_per_second, ticks_per_frame) => {
                1.0 / (frames_per_second.as_f32()
                    * ticks_per_frame as f32)
            }
        };
        Self {
            track,
            timing,
            tick_duration_s,
            next_index: 0,
            next_tick: 0,
            current_time_s: 0.0,
            next_tick_time_s: 0.0,
        }
    }

    fn for_each_new_event<F>(&mut self, ctx: &SigCtx, mut f: F)
    where
        F: FnMut(MidiEvent),
    {
        let time_since_prev_tick_s =
            ctx.num_samples as f32 / ctx.sample_rate_hz;
        self.current_time_s += time_since_prev_tick_s;
        while self.next_tick_time_s < self.current_time_s {
            self.next_tick_time_s += self.tick_duration_s;
            while let Some(event) = self.track.get(self.next_index) {
                let tick = self.next_tick;
                if event.delta == tick {
                    self.next_tick = 0;
                    self.next_index += 1;
                    match event.kind {
                        TrackEventKind::Midi { channel, message } => {
                            f(MidiEvent { channel, message })
                        }
                        TrackEventKind::Meta(MetaMessage::Tempo(
                            us_per_beat,
                        )) => {
                            if let Timing::Metrical(ticks_per_beat) =
                                self.timing
                            {
                                self.tick_duration_s = us_per_beat.as_int()
                                    as f32
                                    / (ticks_per_beat.as_int() as f32
                                        * 1_000_000.0);
                            }
                        }
                        _ => (),
                    }
                } else {
                    break;
                }
            }
            self.next_tick += 1;
        }
    }

    pub fn into_midi_messages(
        mut self,
        channel: u8,
    ) -> FrameSig<impl FrameSigT<Item = MidiMessages>> {
        FrameSig::from_fn(move |ctx| {
            let mut messages = MidiMessages::empty();
            self.for_each_new_event(ctx, |event| {
                if event.channel == u4::new(channel) {
                    messages.push(event.message);
                }
            });
            messages
        })
    }
}
