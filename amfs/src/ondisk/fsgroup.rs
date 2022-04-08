use std::{
    collections::BTreeMap,
    mem,
    ops::{Deref, DerefMut},
    slice,
};

use amos_std::{error::AMErrorFS, AMResult};
use endian_codec::{PackedSize, DecodeLE};
use type_layout::TypeLayout;

use crate::{AMPointerGlobal, Allocator, DiskGroup, LinkedListGlobal, BLOCK_SIZE};

#[repr(C)]
#[derive(Debug, Clone, Copy, TypeLayout)]
/// A group of filesystems.
pub struct FSGroup {
    alloc:       AMPointerGlobal,
    free_queue:  AMPointerGlobal,
    journal:     AMPointerGlobal,
    /// A pointer to the root node of the object tree
    pub objects: AMPointerGlobal,
    directory:   u64,
    txid:        u128,
    _padding:    [u8; BLOCK_SIZE - 88],
}

#[repr(packed)]
/// A list of allocators.
#[derive(Clone, Copy, Debug, PackedSize, DecodeLE)]
pub struct AllocListEntry {
    /// The disk to which the allocator applies
    pub disk_id:   u64,
    /// A pointer to the allocator
    pub allocator: AMPointerGlobal,
}

#[repr(C)]
/// A list of allocators.
#[derive(Clone, Copy, Debug, PackedSize, DecodeLE)]
pub struct FreeQueueEntry {
    /// The txid in which the block was freed
    pub txid:  u128,
    /// A pointer to the block
    pub block: AMPointerGlobal,
}

impl FSGroup {
    /// Creates a new blank FS group
    #[cfg(feature = "unstable")]
    pub fn new() -> FSGroup {
        FSGroup {
            alloc:      AMPointerGlobal::null(),
            free_queue: AMPointerGlobal::null(),
            journal:    AMPointerGlobal::null(),
            objects:    AMPointerGlobal::null(),
            directory:  0,
            txid:       0,
            _padding:   [0; BLOCK_SIZE - 88],
        }
    }
    /// Gets this group's transaction ID
    #[cfg(feature = "stable")]
    pub fn txid(&self) -> u128 {
        self.txid
    }
    /// Gets a pointer to this group's allocator
    #[cfg(feature = "unstable")]
    pub fn alloc(&self) -> AMPointerGlobal {
        self.alloc
    }
    /// Gets a pointer to this group's object set
    #[cfg(feature = "unstable")]
    pub fn objects(&self) -> AMPointerGlobal {
        self.objects
    }
    /// Gets a pointer to this group's journal
    #[cfg(feature = "unstable")]
    pub fn journal(&self) -> AMPointerGlobal {
        self.journal
    }
    /// Gets the object index of this group's directory tree
    #[cfg(feature = "unstable")]
    pub fn directory(&self) -> u64 {
        self.directory
    }
    /// Gets a pointer to this group's free queue
    #[cfg(feature = "unstable")]
    pub fn free_queue(&self) -> AMPointerGlobal {
        self.free_queue
    }
    /// Reads a FSGroup from the disk group
    #[cfg(feature = "unstable")]
    pub fn read(dgs: &[Option<DiskGroup>], ptr: AMPointerGlobal) -> AMResult<FSGroup> {
        if ptr.is_null() {
            return Err(AMErrorFS::NullPointer.into());
        }

        let mut res: FSGroup = FSGroup::new();
        ptr.read(0, BLOCK_SIZE, dgs, &mut res)?;
        assert_or_err!(ptr.validate(dgs)?, AMErrorFS::Checksum);
        Ok(res)
    }
    /// Writes a FSGroup to the disk group
    #[cfg(feature = "unstable")]
    pub fn write(&self, dgs: &[Option<DiskGroup>], ptr: &mut AMPointerGlobal) -> AMResult<()> {
        ptr.write(0, BLOCK_SIZE, dgs, self)?;
        ptr.update(dgs)?;
        Ok(())
    }
    /// Fetches the allocator object for each disk
    #[cfg(feature = "unstable")]
    pub fn get_allocators(&self, dgs: &[Option<DiskGroup>]) -> AMResult<BTreeMap<u64, Allocator>> {
        let allocs: Vec<AllocListEntry> =
            <Vec<AllocListEntry> as LinkedListGlobal<Vec<AllocListEntry>>>::read(dgs, self.alloc)?;
        let mut res = BTreeMap::new();
        for a in allocs {
            debug!("Loaded allocator for disk {:x}", { a.disk_id });
            res.insert(a.disk_id, Allocator::read(dgs, a.allocator)?);
        }
        Ok(res)
    }
    /// Loads the free queue
    #[cfg(feature = "unstable")]
    pub fn get_free_queue(
        &self,
        dgs: &[Option<DiskGroup>],
    ) -> AMResult<BTreeMap<u128, Vec<AMPointerGlobal>>> {
        let queue: Vec<FreeQueueEntry> = <Vec<FreeQueueEntry> as LinkedListGlobal<
            Vec<FreeQueueEntry>,
        >>::read(dgs, self.free_queue)?;
        let mut res = BTreeMap::new();
        for e in queue {
            res.entry(e.txid).or_insert_with(Vec::new).push(e.block);
        }
        Ok(res)
    }
    /// Writes out the free queue
    #[cfg(feature = "unstable")]
    pub fn write_free_queue(
        &mut self,
        dgs: &[Option<DiskGroup>],
        queue: &BTreeMap<u128, Vec<AMPointerGlobal>>,
    ) -> AMResult<()> {
        let mut res = Vec::new();
        for (k, v) in queue {
            for e in v {
                res.push(FreeQueueEntry {
                    txid:  *k,
                    block: *e,
                });
            }
        }
        self.free_queue = LinkedListGlobal::write(&res, dgs, 0)?;
        Ok(())
    }
    /// Writes out the allocator object for each disk
    #[cfg(feature = "unstable")]
    pub fn write_allocators(
        &mut self,
        dgs: &mut [Option<DiskGroup>],
        ad: &mut BTreeMap<u64, Allocator>,
    ) -> AMResult<()> {
        let alloc_blocks = ad
            .iter_mut()
            .map(|(k, v)| Ok((*k, v.prealloc(dgs, 0)?)))
            .collect::<AMResult<BTreeMap<u64, Vec<AMPointerGlobal>>>>()?;
        let allocs: Vec<AllocListEntry> = Vec::new();
        let llg_blocks = LinkedListGlobal::prealloc(&allocs, alloc_blocks.len(), dgs, 0)?;
        let allocs = ad
            .iter_mut()
            .map(|(k, v)| {
                Ok(AllocListEntry {
                    disk_id:   *k,
                    allocator: v.write_preallocd(dgs, &alloc_blocks[k])?,
                })
            })
            .collect::<AMResult<Vec<AllocListEntry>>>()?;
        self.alloc = LinkedListGlobal::write_preallocd(&allocs, dgs, &llg_blocks)?;
        Ok(())
    }
    /// Gets the pointer to the objects table
    #[cfg(feature = "unstable")]
    pub fn get_obj_ptr(&self) -> AMPointerGlobal {
        self.objects
    }
}

impl Deref for FSGroup {
    type Target = [u8];
    #[cfg(feature = "unstable")]
    fn deref(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self as *const FSGroup as *const u8,
                mem::size_of::<FSGroup>(),
            ) as &[u8]
        }
    }
}

impl DerefMut for FSGroup {
    #[cfg(feature = "unstable")]
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self as *mut FSGroup as *mut u8, mem::size_of::<FSGroup>())
                as &mut [u8]
        }
    }
}

#[test]
fn size_test_group() {
    assert_eq!(mem::size_of::<FSGroup>(), BLOCK_SIZE);
}

#[test]
fn size_test_ale() {
    assert_eq!(mem::size_of::<AllocListEntry>(), 24);
}

#[test]
fn size_test_fqe() {
    assert_eq!(mem::size_of::<FreeQueueEntry>(), 32);
}
