use crate::{AMPointerGlobal, DiskGroup, LinkedListGlobal};
use amos_std::AMResult;
use std::collections::BTreeMap;

use std::cell::RefCell;
use std::rc::Rc;

/// A refrence-counted pointer to a disk object
#[derive(Clone, Debug)]
pub struct Allocator(pub Rc<RefCell<AllocatorObj>>);

impl Allocator {
    /// Creates a new allocator
    #[cfg(feature = "stable")]
    pub fn new(size: u64) -> Self {
        Allocator {
            0: Rc::new(RefCell::new(AllocatorObj::new(size))),
        }
    }
    /// Reads a superblock from disk.
    #[cfg(feature = "stable")]
    pub fn read(d: &[Option<DiskGroup>], ptr: AMPointerGlobal) -> AMResult<Self> {
        Ok(Allocator {
            0: Rc::new(RefCell::new(AllocatorObj::read(d, ptr)?)),
        })
    }
    /// Marks an extent used
    #[cfg(feature = "stable")]
    pub fn mark_used(&mut self, start: u64, size: u64) -> AMResult<()> {
        self.0.borrow_mut().mark_used(start, size)
    }
    /// Allocates a contiguous space of a given size
    #[cfg(feature = "stable")]
    pub fn alloc(&mut self, size: u64) -> Option<u64> {
        self.0.borrow_mut().alloc(size)
    }
    /// Allocates several blocks, not necessarily contiguous
    #[cfg(feature = "unstable")]
    pub fn alloc_many(&mut self, count: u64) -> Option<Vec<u64>> {
        self.0.borrow_mut().alloc_many(count)
    }
    /// Writes an allocator to disk.
    #[cfg(feature = "stable")]
    pub fn write(&mut self, d: &mut [Option<DiskGroup>]) -> AMResult<AMPointerGlobal> {
        self.0.borrow_mut().write(d)
    }
    /// Frees a block of space
    #[cfg(feature = "stable")]
    pub fn free(&mut self, start: u64) {
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
    /// Preallocates blocks needed to store the allocator
    #[cfg(feature = "unstable")]
    pub fn prealloc(&self, dgs: &mut [Option<DiskGroup>], n: u8) -> AMResult<Vec<AMPointerGlobal>> {
        let ent_each = (crate::BLOCK_SIZE
            - std::mem::size_of::<crate::ondisk::linkedlist::LLGHeader>())
            / std::mem::size_of::<u64>();
        let exlen = self.0.borrow().extents.len() + 1;
        let blks = if exlen == 0 {
            1
        } else {
            (exlen + (ent_each - 1)) / ent_each
        };
        let mut res = dgs[n as usize].as_mut().ok_or(0)?.alloc_many(blks as u64)?;
        loop {
            let exlen = self.0.borrow().extents.len();
            let blks = if exlen == 0 {
                1
            } else {
                (exlen + (ent_each - 1)) / ent_each
            };
            if res.len() >= blks {
                break;
            }
            res.append(
                &mut dgs[n as usize]
                    .as_mut()
                    .ok_or(0)?
                    .alloc_many((blks - res.len()) as u64)?,
            )
        }
        Ok(res)
    }
    /// Writes out the allocator into a preallocated set of blocks
    #[cfg(feature = "unstable")]
    pub fn write_preallocd(
        &mut self,
        dgs: &mut [Option<DiskGroup>],
        ptrs: &[AMPointerGlobal],
    ) -> AMResult<AMPointerGlobal> {
        self.0.borrow_mut().write_preallocd(dgs, ptrs)
    }
}

/// The filesystem's block allocator
#[derive(Debug, PartialEq)]
pub struct AllocatorObj {
    size: u64,
    extents: BTreeMap<u64, Extent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Extent {
    size: u64,
    used: bool,
}

impl AllocatorObj {
    #[cfg(feature = "stable")]
    fn new(size: u64) -> Self {
        let mut exmap = BTreeMap::new();
        exmap.insert(0, Extent { size, used: false });
        Self {
            size,
            extents: exmap,
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
    fn alloc(&mut self, size: u64) -> Option<u64> {
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
                return Some(*a);
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
            *self.extents.get_mut(&a).unwrap() = Extent {
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
            return Some(a);
        }
        None
    }
    #[cfg(feature = "unstable")]
    fn alloc_many(&mut self, count: u64) -> Option<Vec<u64>> {
        let mut res = Vec::new();
        for _ in 0..count {
            if let Some(v) = self.alloc(1) {
                res.push(v);
            } else {
                for a in res {
                    self.free(a);
                }
                return None;
            }
        }
        Some(res)
    }
    #[cfg(feature = "stable")]
    fn free(&mut self, addr: u64) {
        let ex = self.extents.get_mut(&addr).unwrap();
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
            self.extents.get_mut(&addr).unwrap().size += n_s;
            ex.size += n_s;
            self.extents.remove(&n_a);
        }
        if let Some(p_a) = merge_previous {
            self.extents.get_mut(&p_a).unwrap().size += ex.size;
            self.extents.remove(&addr);
        }
    }
    #[cfg(feature = "stable")]
    fn mark_used(&mut self, start: u64, size: u64) -> AMResult<()> {
        let containing = self.extents.range(..=start).next_back();
        if containing.is_none() {
            panic!("No containing extent");
        }
        assert!(!containing.ok_or(0)?.1.used);
        let c = (*containing.ok_or(0)?.0, containing.ok_or(0)?.1.size);
        assert!(c.0 + c.1 >= start + size);
        if start == c.0 {
            if c.1 == size {
                self.extents.get_mut(&c.0).ok_or(0)?.used = true;
            } else {
                let ex = self.extents.get_mut(&c.0).ok_or(0)?;
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
            let ex = self.extents.get_mut(&c.0).ok_or(0)?;
            ex.size -= size;
            self.extents.insert(start, Extent { size, used: true });
        } else {
            let ex = self.extents.get_mut(&c.0).unwrap();
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
    fn read(dgs: &[Option<DiskGroup>], ptr: AMPointerGlobal) -> AMResult<Self> {
        let a = <Vec<u64> as LinkedListGlobal<Vec<u64>>>::read(dgs, ptr)?;
        let mut start = 0;
        let mut allocator = Self::new(a[0]);
        for l in a[1..].iter() {
            let size = l & 0x7FFFFFFFFFFFFFFF;
            let used = (l & 0x8000000000000000) != 0;
            allocator.extents.insert(start, Extent { size, used });
            start += size;
        }
        Ok(allocator)
    }
    #[cfg(feature = "unstable")]
    fn write(&mut self, dgs: &mut [Option<DiskGroup>]) -> AMResult<AMPointerGlobal> {
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
        LinkedListGlobal::write(&a, dgs, 0)
    }
    #[cfg(feature = "unstable")]
    fn write_preallocd(
        &mut self,
        dgs: &mut [Option<DiskGroup>],
        ptrs: &[AMPointerGlobal],
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
        LinkedListGlobal::write_preallocd(&a, dgs, ptrs)
    }
}

#[test]
fn rw_test() {
    #![allow(clippy::unwrap_used)]
    use rand::Rng;

    let dg = crate::test::dg::create_dg_mem_single(10000);

    let mut a = AllocatorObj::new(10005);

    for _ in 0..2000 {
        a.alloc(rand::thread_rng().gen_range(1..5));
    }

    a.mark_used(10000, 5).unwrap();

    let ptr = a.write(&mut vec![Some(dg.clone())]).unwrap();

    let a2 = AllocatorObj::read(&vec![Some(dg)], ptr).unwrap();

    assert_eq!(a, a2);
}
