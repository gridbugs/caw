use lazy_static::lazy_static;
use midly::num::{u4, u7};
use std::{collections::HashMap, sync::Mutex};

struct ChannelAllocator {
    next_channel: Option<u4>,
    next_controller_index_by_channel: HashMap<u4, Option<u7>>,
}

impl ChannelAllocator {
    fn new() -> Self {
        Self {
            next_channel: Some(0.into()),
            next_controller_index_by_channel: HashMap::new(),
        }
    }

    fn alloc_channel(&mut self) -> u4 {
        let channel = self.next_channel.expect("Can't allocate MIDI channel because all MIDI channels are in use already.");
        self.next_channel = if channel == u4::max_value() {
            None
        } else {
            Some((u8::from(channel) + 1).into())
        };
        channel
    }

    fn alloc_controller(&mut self, channel: u4) -> u7 {
        if let Some(next_channel) = self.next_channel {
            assert!(
                channel < next_channel,
                "Attempted to allocate controller on unallocated channel."
            );
        }
        let next_controller = self
            .next_controller_index_by_channel
            .entry(channel)
            .or_insert_with(|| Some(0.into()));
        if let Some(controller) = *next_controller {
            *next_controller = if controller == u7::max_value() {
                None
            } else {
                Some((u8::from(controller) + 1).into())
            };
            controller
        } else {
            panic!(
                "Can't allocate MIDI controller because all MIDI controllers on channel {} are in use already.",
                channel
            );
        }
    }
}

lazy_static! {
    static ref CHANNEL_ALLOCATOR: Mutex<ChannelAllocator> =
        Mutex::new(ChannelAllocator::new());
}

pub fn alloc_channel() -> u4 {
    CHANNEL_ALLOCATOR.lock().unwrap().alloc_channel()
}

pub fn alloc_controller(channel: u4) -> u7 {
    let mut allocator = CHANNEL_ALLOCATOR.lock().unwrap();
    allocator.alloc_controller(channel)
}
