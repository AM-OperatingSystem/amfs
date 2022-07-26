use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use amos_std::{
    error::{AMError, AMErrorFS},
    AMResult,
};

use crate::{AMPointerGlobal, DiskGroup, LinkedListGlobal};

/// A reference-counted pointer to a disk object
#[derive(Clone, Debug)]
pub struct Allocator(pub Rc<RefCell<AllocatorObj>>);

impl Allocator {
    /// Creates a new allocator
    #[cfg(feature = "stable")]
    pub fn new(size: u64) -> Self {
        Allocator(Rc::new(RefCell::new(AllocatorObj::new(size))))
    }
    /// Reads a superblock from disk.
    #[cfg(feature = "stable")]
    pub fn read(d: &[Option<DiskGroup>], ptr: AMPointerGlobal) -> AMResult<Self> {
        Ok(Allocator(Rc::new(RefCell::new(AllocatorObj::read(
            d, ptr,
        )?))))
    }
    /// Marks an extent used
    #[cfg(feature = "stable")]
    pub fn mark_used(&mut self, start: u64, size: u64) -> AMResult<()> {
        self.0.borrow_mut().mark_used(start, size)
    }
    /// Allocates a contiguous space of a given size
    #[cfg(feature = "stable")]
    pub fn alloc_blocks(&mut self, size: u64) -> AMResult<u64> {
        self.0.borrow_mut().alloc(size)
    }
    /// Allocates several blocks, not necessarily contiguous
    #[cfg(feature = "unstable")]
    pub fn alloc_many(&mut self, count: u64) -> AMResult<Vec<u64>> {
        self.0.borrow_mut().alloc_many(count)
    }
    /// Writes an allocator to disk.
    #[cfg(feature = "stable")]
    pub fn write(&mut self, d: &mut [Option<DiskGroup>]) -> AMResult<AMPointerGlobal> {
        self.0.borrow_mut().write(d)
    }
    /// Frees a block of space
    #[cfg(feature = "stable")]
    pub fn free(&mut self, start: u64) -> AMResult<()> {
        self.0.borrow_mut().free(start)
    }
    /// Returns the amount of space free
    #[cfg(feature = "stable")]
    pub fn free_space(&self) -> u64 {
        self.0.borrow().free_space()
    }
    /// Returns the amount of space in use
    #[cfg(feature = "stable")]
    pub fn used_space(&self) -> u64 {
        self.0.borrow().used_space()
    }
    /// Returns the total space belonging to this allocator
    #[cfg(feature = "stable")]
    pub fn total_space(&self) -> u64 {
        self.0.borrow().total_space()
    }
    /// Gets the list of extents
    #[cfg(feature = "unstable")]
    pub fn extents(&self) -> BTreeMap<u64, Extent> {
        self.0.borrow().extents.clone()
    }
    /// Preallocates blocks needed to store the allocator
    #[cfg(feature = "unstable")]
    pub fn prealloc(
        &self,
        diskgroups: &mut [Option<DiskGroup>],
        n: u8,
    ) -> AMResult<Vec<AMPointerGlobal>> {
        let extents_per_block = (crate::BLOCK_SIZE
            - std::mem::size_of::<crate::ondisk::linkedlist::LLGHeader>())
            / std::mem::size_of::<u64>();
        let extents = self.0.borrow().extents.len() + 1;
        let blocks = if extents == 0 {
            1
        } else {
            (extents + (extents_per_block - 1)) / extents_per_block
        };
        let mut res = diskgroups[n as usize]
            .as_mut()
            .ok_or(AMErrorFS::NoDiskgroup)?
            .alloc_many(blocks as u64)?;
        loop {
            let extents = self.0.borrow().extents.len();
            let blocks = if extents == 0 {
                1
            } else {
                (extents + (extents_per_block - 1)) / extents_per_block
            };
            if res.len() >= blocks {
                break;
            }
            res.append(
                &mut diskgroups[n as usize]
                    .as_mut()
                    .ok_or(AMErrorFS::NoDiskgroup)?
                    .alloc_many((blocks - res.len()) as u64)?,
            )
        }
        Ok(res)
    }
    /// Writes out the allocator into a preallocated set of blocks
    #[cfg(feature = "unstable")]
    pub fn write_preallocd(
        &mut self,
        diskgroups: &mut [Option<DiskGroup>],
        blocks: &[AMPointerGlobal],
    ) -> AMResult<AMPointerGlobal> {
        self.0.borrow_mut().write_preallocd(diskgroups, blocks)
    }
}

/// The filesystem's block allocator
#[derive(Debug, PartialEq, Eq)]
pub struct AllocatorObj {
    size:    u64,
    extents: BTreeMap<u64, Extent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extent {
    pub size: u64,
    pub used: bool,
}

impl AllocatorObj {
    #[cfg(feature = "stable")]
    fn new(size: u64) -> Self {
        let mut extent_map = BTreeMap::new();
        extent_map.insert(0, Extent { size, used: false });
        Self {
            size,
            extents: extent_map,
        }
    }
    /// Returns the amount of space free
    #[cfg(feature = "stable")]
    fn free_space(&self) -> u64 {
        let mut free = 0;
        for ex in self.extents.values() {
            if !ex.used {
                free += ex.size;
            }
        }
        free
    }
    /// Returns the amount of space in use
    #[cfg(feature = "stable")]
    fn used_space(&self) -> u64 {
        let mut used = 0;
        for ex in self.extents.values() {
            if ex.used {
                used += ex.size;
            }
        }
        used
    }
    /// Returns the total space belonging to this allocator
    #[cfg(feature = "stable")]
    fn total_space(&self) -> u64 {
        self.size
    }
    #[cfg(feature = "stable")]
    fn alloc(&mut self, size: u64) -> AMResult<u64> {
        assert!(size > 0);
        assert_le!(size, self.size);
        trace!("Allocating block of size: {:x}", size);
        for (a, ex) in self.extents.iter_mut() {
            if ex.used {
                continue;
            }
            if ex.size == size {
                trace!("Found exact match");
                ex.used = true;
                return Ok(*a);
            }
        }
        let mut exs = None;
        for (a, ex) in self.extents.iter_mut() {
            if ex.used {
                continue;
            }
            if ex.size > size {
                trace!("Found larger extent: {:x}", ex.size);
                exs = Some((*a, size, ex.size));
                break;
            }
        }
        if let Some((a, sa, se)) = exs {
            *self.extents.get_mut(&a).ok_or(AMError::TODO(0))? = Extent {
                size: sa,
                used: true,
            };
            self.extents.insert(
                a + sa,
                Extent {
                    size: se - sa,
                    used: false,
                },
            );
            return Ok(a);
        }
        Err(AMErrorFS::AllocFailed.into())
    }
    #[cfg(feature = "unstable")]
    fn alloc_many(&mut self, count: u64) -> AMResult<Vec<u64>> {
        let mut res = Vec::new();
        for _ in 0..count {
            if let Ok(v) = self.alloc(1) {
                res.push(v);
            } else {
                for a in res {
                    self.free(a)
                        .unwrap_or_else(|_| panic!("Failed to free after failed allocation"));
                }
                return Err(AMErrorFS::AllocFailed.into());
            }
        }
        Ok(res)
    }
    #[cfg(feature = "stable")]
    fn free(&mut self, addr: u64) -> AMResult<()> {
        let ex = self.extents.get_mut(&addr).ok_or(AMError::TODO(0))?;
        assert!(ex.used);
        ex.used = false;
        let mut ex = ex.clone(); //Make a copy here to free the extent map;
        let mut merge_previous = None;
        let mut merge_next = None;
        if let Some(p) = self.extents.range(..addr).next_back() {
            if !p.1.used {
                merge_previous = Some(*p.0)
            }
        }
        if let Some(n) = self.extents.range(addr..).nth(1) {
            if !n.1.used {
                merge_next = Some((*n.0, n.1.size))
            }
        }
        if let Some((n_a, n_s)) = merge_next {
            self.extents.get_mut(&addr).ok_or(AMError::TODO(0))?.size += n_s;
            ex.size += n_s;
            self.extents.remove(&n_a);
        }
        if let Some(p_a) = merge_previous {
            self.extents.get_mut(&p_a).ok_or(AMError::TODO(0))?.size += ex.size;
            self.extents.remove(&addr);
        }
        Ok(())
    }
    #[cfg(feature = "stable")]
    fn mark_used(&mut self, start: u64, size: u64) -> AMResult<()> {
        let containing = self.extents.range(..=start).next_back();
        if containing.is_none() {
            panic!("No containing extent");
        }
        assert!(!containing.ok_or(AMError::TODO(0))?.1.used);
        let c = (
            *containing.ok_or(AMError::TODO(0))?.0,
            containing.ok_or(AMError::TODO(0))?.1.size,
        );
        assert!(c.0 + c.1 >= start + size);
        if start == c.0 {
            if c.1 == size {
                self.extents.get_mut(&c.0).ok_or(AMError::TODO(0))?.used = true;
            } else {
                let ex = self.extents.get_mut(&c.0).ok_or(AMError::TODO(0))?;
                ex.used = true;
                ex.size = size;
                self.extents.insert(
                    c.0 + size,
                    Extent {
                        size: c.1 - size,
                        used: false,
                    },
                );
            }
        } else if c.0 + c.1 == start + size {
            let ex = self.extents.get_mut(&c.0).ok_or(AMError::TODO(0))?;
            ex.size -= size;
            self.extents.insert(start, Extent { size, used: true });
        } else {
            let ex = self.extents.get_mut(&c.0).ok_or(AMError::TODO(0))?;
            ex.size = start - c.0;
            self.extents.insert(start, Extent { size, used: true });
            self.extents.insert(
                start + size,
                Extent {
                    size: (c.0 + c.1) - (start + size),
                    used: false,
                },
            );
        }
        Ok(())
    }
    #[cfg(feature = "stable")]
    fn read(diskgroups: &[Option<DiskGroup>], ptr: AMPointerGlobal) -> AMResult<Self> {
        let a = <Vec<u64> as LinkedListGlobal<Vec<u64>>>::read(diskgroups, ptr)?;
        let mut start = 0;
        let size = *a.first().ok_or(AMErrorFS::NoAllocator)?;
        let mut allocator = Self::new(size);
        for l in a[1..].iter() {
            let size = l & 0x7FFFFFFFFFFFFFFF;
            let used = (l & 0x8000000000000000) != 0;
            allocator.extents.insert(start, Extent { size, used });
            start += size;
        }
        Ok(allocator)
    }
    #[cfg(feature = "unstable")]
    fn write(&mut self, diskgroups: &mut [Option<DiskGroup>]) -> AMResult<AMPointerGlobal> {
        let mut a: Vec<u64> = self
            .extents
            .values()
            .map(|v| {
                if v.used {
                    v.size | 0x8000000000000000
                } else {
                    v.size
                }
            })
            .collect();
        a.insert(0, self.size);
        LinkedListGlobal::write(&a, diskgroups, 0)
    }
    #[cfg(feature = "unstable")]
    fn write_preallocd(
        &mut self,
        diskgroups: &mut [Option<DiskGroup>],
        blocks: &[AMPointerGlobal],
    ) -> AMResult<AMPointerGlobal> {
        let mut a: Vec<u64> = self
            .extents
            .values()
            .map(|v| {
                if v.used {
                    v.size | 0x8000000000000000
                } else {
                    v.size
                }
            })
            .collect();
        a.insert(0, self.size);
        LinkedListGlobal::write_preallocd(&a, diskgroups, blocks)
    }
}

#[test]
fn rw_test() {
    #![allow(clippy::unwrap_used)]
    use rand::Rng;

    let dg = crate::test::dg::create_dg_mem_single(10000);

    let mut a = AllocatorObj::new(10005);

    for _ in 0..2000 {
        a.alloc(rand::thread_rng().gen_range(1..5)).unwrap();
    }

    a.mark_used(10000, 5).unwrap();

    let ptr = a.write(&mut vec![Some(dg.clone())]).unwrap();

    let a2 = AllocatorObj::read(&vec![Some(dg)], ptr).unwrap();

    assert_eq!(a, a2);
}
