use std::{mem,slice};
use std::ops::{Deref,DerefMut};

use std::collections::BTreeMap;

use crate::{AMPointerGlobal,DiskGroup,Allocator,LinkedListGlobal};
use amos_std::AMResult;

use crate::BLOCK_SIZE;

#[repr(packed)]
/// A group of filesystems.
pub struct FSGroup {
    alloc: AMPointerGlobal,
    _journal: AMPointerGlobal,
    /// A pointer to the root node of the object tree
    pub objects: AMPointerGlobal,
    _directory: AMPointerGlobal,
    txid: u128,
    _padding: [u8; BLOCK_SIZE - 80],
}

#[repr(packed)]
/// A list of allocators.
#[derive(Clone,Copy)]
pub struct AllocListEntry {
    disk_id: u64,
    allocator: AMPointerGlobal,
}

impl FSGroup {
    /// Creates a new blank FS group
    pub fn new() -> FSGroup {
        FSGroup {
            alloc: AMPointerGlobal::null(),
            _journal: AMPointerGlobal::null(),
            objects: AMPointerGlobal::null(),
            _directory: AMPointerGlobal::null(),
            txid: 0,
            _padding: [0; BLOCK_SIZE - 80],
        }
    }
    /// Gets this group's transaction ID
    pub fn get_txid(&self) -> u128 {
        self.txid
    }
    /// Reads a FSGroup from the disk group
    pub fn read(dgs: &[Option<DiskGroup>], ptr: AMPointerGlobal) -> AMResult<FSGroup> {
        assert!(!ptr.is_null());
        
        let mut res: FSGroup = FSGroup::new();
        ptr.read(0,BLOCK_SIZE,dgs, &mut res)?;
        assert!(ptr.validate(dgs)?);
        Ok(res)
    }
    /// Writes a FSGroup to the disk group
    pub fn write(&self, dgs: &[Option<DiskGroup>], n: u8) -> AMResult<AMPointerGlobal> {
        let mut dg = dgs[n as usize].as_ref().ok_or(0)?.clone();
        let mut ptr = dg.alloc(1)?;
        ptr.write(0,BLOCK_SIZE,dgs, self)?;
        ptr.update(dgs)?;
        Ok(ptr)
    }
    /// Fetches the allocator object for each disk
    pub fn get_allocators(&self, dgs: &[Option<DiskGroup>]) -> AMResult<BTreeMap<u64,Allocator>> {
        let allocs : Vec<AllocListEntry> = <Vec<AllocListEntry> as LinkedListGlobal<Vec<AllocListEntry>>>::read(dgs,self.alloc)?;
        let mut res = BTreeMap::new();
        for a in allocs {
            debug!("Loaded allocator for disk {:x}",{a.disk_id});
            res.insert(a.disk_id,Allocator::read(dgs,a.allocator)?);
        }
        Ok(res)
    }
    /// Writes out the allocator object for each disk
    pub fn write_allocators(&mut self, dgs: &mut [Option<DiskGroup>], ad: &mut BTreeMap<u64,Allocator>) -> AMResult<()> {
        let alloc_blocks = ad.iter_mut().map(|(k,v)| Ok((*k,v.prealloc(dgs,0)?))).collect::<AMResult<BTreeMap<u64, Vec<AMPointerGlobal>>>>()?;
        let allocs : Vec<AllocListEntry> = Vec::new();
        let llg_blocks = LinkedListGlobal::prealloc(&allocs,alloc_blocks.len(),dgs,0)?;
        let allocs = ad.iter_mut().map(|(k,v)| Ok(AllocListEntry{disk_id:*k,allocator:v.write_preallocd(dgs,&alloc_blocks[k])?})).collect::<AMResult<Vec<AllocListEntry>>>()?;
        self.alloc = LinkedListGlobal::write_preallocd(&allocs, dgs,&llg_blocks)?;
        Ok(())
    }
    /// Gets the pointer to the objects table
    pub fn get_obj_ptr(&self) -> AMPointerGlobal {
        self.objects
    }
}

impl Deref for FSGroup {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self as *const FSGroup as *const u8, mem::size_of::<FSGroup>())
                as &[u8]
        }
    }
}

impl DerefMut for FSGroup {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self as *mut FSGroup as *mut u8, mem::size_of::<FSGroup>())
                as &mut [u8]
        }
    }
}

impl Default for FSGroup {
    fn default() -> Self {
        FSGroup::new()
    }
}

#[test]
fn size_test_group() {
    assert_eq!(mem::size_of::<FSGroup>(), BLOCK_SIZE);
}