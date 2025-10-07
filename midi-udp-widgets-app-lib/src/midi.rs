use lazy_static::lazy_static;
use midly::num::{u4, u7};
use std::sync::Mutex;

pub type MidiChannel = u4;
type MidiResource = u7;
pub type MidiController = MidiResource;
pub type MidiKey = MidiResource;

#[derive(Debug)]
struct AllMidiChannelsAreAlreadyAllocated;

#[derive(Debug)]
struct AllMidiResourcesAreAlreadyAllocated;

struct GenericAllocator {
    next: Option<u8>,
    max: u8,
}

impl GenericAllocator {
    fn new(max: u8) -> Self {
        Self { next: Some(0), max }
    }

    fn alloc(&mut self) -> Option<u8> {
        self.next.map(|next| {
            self.next = if next == self.max {
                None
            } else {
                Some(next + 1)
            };
            next
        })
    }
}

struct MidiChannelAllocator(GenericAllocator);

impl MidiChannelAllocator {
    fn new() -> Self {
        Self(GenericAllocator::new(u4::max_value().into()))
    }

    fn alloc(
        &mut self,
    ) -> Result<MidiChannel, AllMidiChannelsAreAlreadyAllocated> {
        match self.0.alloc() {
            Some(value) => Ok(value.into()),
            None => Err(AllMidiChannelsAreAlreadyAllocated),
        }
    }
}

/// Allocator for 7-bit midi resources such as controllers and keys
struct MidiResourceAllocator(GenericAllocator);
impl MidiResourceAllocator {
    fn new() -> Self {
        Self(GenericAllocator::new(u7::max_value().into()))
    }

    fn alloc(
        &mut self,
    ) -> Result<MidiResource, AllMidiResourcesAreAlreadyAllocated> {
        match self.0.alloc() {
            Some(value) => Ok(value.into()),
            None => Err(AllMidiResourcesAreAlreadyAllocated),
        }
    }
}

impl Default for MidiResourceAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug)]
enum MidiResourceType {
    Controller,
    Key,
}

#[derive(Default)]
struct MidiResourceTable<T> {
    controller: T,
    key: T,
}

impl<T> MidiResourceTable<T> {
    fn get_mut(&mut self, type_: MidiResourceType) -> &mut T {
        match type_ {
            MidiResourceType::Controller => &mut self.controller,
            MidiResourceType::Key => &mut self.key,
        }
    }

    fn get(&self, type_: MidiResourceType) -> &T {
        match type_ {
            MidiResourceType::Controller => &self.controller,
            MidiResourceType::Key => &self.key,
        }
    }
}

/// Allocates resources associated with a particular channel
struct MidiResourceAllocators {
    resource_table: MidiResourceTable<MidiResourceAllocator>,
    channel: MidiChannel,
}

impl MidiResourceAllocators {
    fn new(channel: MidiChannel) -> Self {
        Self {
            resource_table: Default::default(),
            channel,
        }
    }
}

/// Allocates resources (controllers, keys) or entire channels. Resources are allocated from a pool
/// of channels managed internally by this type.
struct MidiAllocator {
    channel_allocator: MidiChannelAllocator,
    resource_allocators_pool: Vec<MidiResourceAllocators>,
    resource_pool_indices: MidiResourceTable<usize>,
}

impl MidiAllocator {
    fn new() -> Self {
        let mut channel_allocator = MidiChannelAllocator::new();
        let resource_allocator = {
            let channel = channel_allocator
                .alloc()
                .expect("First allocation should always succeed.");
            MidiResourceAllocators::new(channel)
        };
        Self {
            channel_allocator,
            resource_allocators_pool: vec![resource_allocator],
            resource_pool_indices: MidiResourceTable {
                controller: 0,
                key: 0,
            },
        }
    }

    fn alloc_channel(
        &mut self,
    ) -> Result<MidiChannel, AllMidiChannelsAreAlreadyAllocated> {
        self.channel_allocator.alloc()
    }

    fn alloc_resource(
        &mut self,
        type_: MidiResourceType,
    ) -> Result<(MidiChannel, MidiResource), AllMidiChannelsAreAlreadyAllocated>
    {
        if let Some(resource_allocators) = self
            .resource_allocators_pool
            .get_mut(*self.resource_pool_indices.get(type_))
        {
            match resource_allocators.resource_table.get_mut(type_).alloc() {
                Ok(resource_index) => {
                    Ok((resource_allocators.channel, resource_index))
                }
                Err(AllMidiResourcesAreAlreadyAllocated) => {
                    let resource_pool_index =
                        self.resource_pool_indices.get_mut(type_);
                    *resource_pool_index += 1;
                    let resource_allocators = if let Some(resource_allocators) =
                        self.resource_allocators_pool
                            .get_mut(*resource_pool_index)
                    {
                        resource_allocators
                    } else {
                        let channel = self.channel_allocator.alloc()?;
                        self.resource_allocators_pool
                            .push(MidiResourceAllocators::new(channel));
                        assert_eq!(
                            *resource_pool_index,
                            self.resource_allocators_pool.len()
                        );
                        self.resource_allocators_pool.last_mut().unwrap()
                    };
                    let resource_index = resource_allocators
                        .resource_table
                        .get_mut(type_)
                        .alloc()
                        .expect("First allocation should always succeed.");
                    Ok((resource_allocators.channel, resource_index))
                }
            }
        } else {
            panic!("Resource pool index should always be a valid index.");
        }
    }

    /// Allocate a pair of resources from the same channel. If the first resource would be the
    /// final resource from its channel, it is leaked and a pair of resources are allocated from
    /// the subsequent channel instead.
    fn alloc_resource_2(
        &mut self,
        type_: MidiResourceType,
    ) -> Result<
        (MidiChannel, MidiResource, MidiResource),
        AllMidiChannelsAreAlreadyAllocated,
    > {
        let r0 = self.alloc_resource(type_)?;
        let r1 = self.alloc_resource(type_)?;
        if r0.0 == r1.0 {
            Ok((r0.0, r0.1, r1.1))
        } else {
            let r2 = self.alloc_resource(type_)?;
            assert_eq!(r1.0, r2.0);
            Ok((r1.0, r1.1, r2.1))
        }
    }

    fn alloc_controller(
        &mut self,
    ) -> Result<(MidiChannel, MidiController), AllMidiChannelsAreAlreadyAllocated>
    {
        self.alloc_resource(MidiResourceType::Controller)
    }

    fn alloc_controller_2(
        &mut self,
    ) -> Result<
        (MidiChannel, MidiController, MidiController),
        AllMidiChannelsAreAlreadyAllocated,
    > {
        self.alloc_resource_2(MidiResourceType::Controller)
    }

    fn alloc_key(
        &mut self,
    ) -> Result<(MidiChannel, MidiKey), AllMidiChannelsAreAlreadyAllocated>
    {
        self.alloc_resource(MidiResourceType::Key)
    }
}

lazy_static! {
    static ref ALLOCATOR: Mutex<MidiAllocator> =
        Mutex::new(MidiAllocator::new());
}

pub fn alloc_channel() -> MidiChannel {
    ALLOCATOR
        .lock()
        .unwrap()
        .alloc_channel()
        .expect("All MIDI channels are already allocated.")
}

pub fn alloc_controller() -> (MidiChannel, MidiController) {
    ALLOCATOR.lock().unwrap().alloc_controller().expect(
        "Allocating a new controller would require allocating a new channel, \
        but all MIDI channels are already allocated.",
    )
}

pub fn alloc_controller_2() -> (MidiChannel, MidiController, MidiController) {
    ALLOCATOR.lock().unwrap().alloc_controller_2().expect(
        "Allocating a new controller would require allocating a new channel, \
        but all MIDI channels are already allocated.",
    )
}

pub fn alloc_key() -> (MidiChannel, MidiKey) {
    ALLOCATOR.lock().unwrap().alloc_key().expect(
        "Allocating a new controller would require allocating a new key, \
        but all MIDI channels are already allocated.",
    )
}
