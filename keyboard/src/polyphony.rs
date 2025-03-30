use crate::{KeyEvent, KeyEvents, MonoVoice, Note};
use caw_core::{sig_shared, Sig, SigT};
use smallvec::SmallVec;
use std::collections::BinaryHeap;

#[derive(PartialEq, Eq)]
struct FreeVoice {
    release_at_batch_index: u64,
    index: usize,
}

struct UsedVoice {
    note: Note,
    index: usize,
}

/// Comparison where lower released times are treated as higher values, and any released values is
/// higher than any pressed value. This is purely so that a max heap (`BinaryHeap`) can be used to
/// allocate the earliest released voice.
mod free_voice_cmp {
    use super::FreeVoice;
    use std::cmp::Ordering;
    impl PartialOrd for FreeVoice {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for FreeVoice {
        fn cmp(&self, other: &Self) -> Ordering {
            other
                .release_at_batch_index
                .cmp(&self.release_at_batch_index)
        }
    }
}

/// There are a fixed number of voices. `VoiceAllocator` allocates the free voice which was freed
/// least recently of all free voices so that released notes are given the longest amount of time
/// to continue producing sound before being reused to service new key presses.
struct VoiceAllocator {
    free_voices: BinaryHeap<FreeVoice>,
    used_voices: Vec<UsedVoice>,
}

impl VoiceAllocator {
    fn new(n: usize) -> Self {
        let free_voices = {
            let mut free_voices = BinaryHeap::new();
            for index in 0..n {
                free_voices.push(FreeVoice {
                    release_at_batch_index: 0,
                    index,
                });
            }
            free_voices
        };
        Self {
            free_voices,
            used_voices: Vec::new(),
        }
    }

    fn alloc(&mut self, note: Note) -> Option<usize> {
        self.free_voices.pop().map(|FreeVoice { index, .. }| {
            self.used_voices.push(UsedVoice { note, index });
            index
        })
    }

    /// Note that this may free multiple voices in a single call. This can happen if the requested
    /// note is currently being played on multiple different voices.
    fn free(
        &mut self,
        note: Note,
        batch_index: u64,
        freed_indices: &mut Vec<usize>,
    ) {
        self.used_voices.retain(|used_voice| {
            if used_voice.note == note {
                self.free_voices.push(FreeVoice {
                    release_at_batch_index: batch_index,
                    index: used_voice.index,
                });
                freed_indices.push(used_voice.index);
                false
            } else {
                true
            }
        });
    }
}

#[derive(Clone)]
struct PolyKeyEvent {
    key_event: KeyEvent,
    voice_index: usize,
}

/// Routes key events to voice indices
fn route_key_events<K>(
    key_events: K,
    n: usize,
) -> Sig<impl SigT<Item = SmallVec<[PolyKeyEvent; 1]>>>
where
    K: SigT<Item = KeyEvents>,
{
    let mut voice_allocator = VoiceAllocator::new(n);
    let mut to_free = Vec::new();
    Sig(key_events).map_mut_ctx(move |key_events, ctx| {
        let mut out: SmallVec<[PolyKeyEvent; 1]> = smallvec::smallvec![];
        for key_event in key_events {
            if key_event.pressed {
                if let Some(voice_index) = voice_allocator.alloc(key_event.note)
                {
                    out.push(PolyKeyEvent {
                        key_event,
                        voice_index,
                    });
                } else {
                    log::warn!("Unable to allocate voice for note!");
                }
            } else {
                voice_allocator.free(
                    key_event.note,
                    ctx.batch_index,
                    &mut to_free,
                );
                for voice_index in to_free.drain(..) {
                    out.push(PolyKeyEvent {
                        key_event,
                        voice_index,
                    });
                }
            }
        }
        out
    })
}

/// Return a vector with n monophonic voices that can be played together to polyphonically
/// represent the stream of key events `key_events`.
pub fn voices_from_key_events<K>(
    key_events: K,
    n: usize,
) -> Vec<MonoVoice<impl SigT<Item = KeyEvents>>>
where
    K: SigT<Item = KeyEvents>,
{
    let poly_key_events = sig_shared(route_key_events(key_events, n));
    (0..n)
        .map(|i| {
            let key_events_for_this_voice =
                poly_key_events.clone().map(move |poly_key_events| {
                    let mut out = KeyEvents::empty();
                    for PolyKeyEvent {
                        key_event,
                        voice_index,
                    } in poly_key_events
                    {
                        if voice_index == i {
                            out.push(key_event);
                        }
                    }
                    out
                });
            MonoVoice::from_key_events(key_events_for_this_voice)
        })
        .collect()
}
