use std::convert::TryFrom;

use amos_std::{error::AMErrorFS, AMResult};
use endian_codec::{DecodeLE, PackedSize};

use crate::{any_as_u8_slice, u8_slice_as_any, AMPointerGlobal, DiskGroup, BLOCK_SIZE};

#[repr(C)]
#[derive(PackedSize, DecodeLE)]
pub(crate) struct LLGHeader {
    next:     AMPointerGlobal,
    count:    u64,
    _padding: u64,
}

/// Trait for writing a collection of items to disk, using global pointers.
pub trait LinkedListGlobal<T: Sized> {
    /// Reads the linked list from disk
    fn read(d: &[Option<DiskGroup>], p: AMPointerGlobal) -> AMResult<T>;
    /// Writes the linked list to disk
    fn write(&self, d: &[Option<DiskGroup>], n: u8) -> AMResult<AMPointerGlobal>;
    /// Writes the linked list to disk, using previously allocated blocks
    fn prealloc(
        &self,
        count: usize,
        d: &mut [Option<DiskGroup>],
        n: u8,
    ) -> AMResult<Vec<AMPointerGlobal>>;
    /// Writes the linked list to disk, using previously allocated blocks
    fn write_preallocd(
        &self,
        d: &[Option<DiskGroup>],
        blocks: &[AMPointerGlobal],
    ) -> AMResult<AMPointerGlobal>;
}

impl<T: Copy + std::fmt::Debug + DecodeLE> LinkedListGlobal<Vec<T>> for Vec<T> {
    #[cfg(feature = "unstable")]
    fn read(diskgroups: &[Option<DiskGroup>], mut p: AMPointerGlobal) -> AMResult<Vec<T>> {
        let mut res = Vec::new();
        let mut buf = [0; BLOCK_SIZE];
        loop {
            if p.is_null() {
                break;
            }
            let count;
            assert!(p.validate(diskgroups)?);
            p.read(0, BLOCK_SIZE, diskgroups, &mut buf)?;
            unsafe {
                let hdr = u8_slice_as_any::<LLGHeader>(&buf);
                p = hdr.next;
                count = hdr.count;
            }
            for i in 0..usize::try_from(count)? {
                unsafe {
                    let addr = std::mem::size_of::<LLGHeader>() + std::mem::size_of::<T>() * i;
                    let ent = u8_slice_as_any::<T>(&buf[addr..]);
                    res.push(ent);
                }
            }
        }
        Ok(res)
    }
    #[cfg(feature = "unstable")]
    fn write(&self, diskgroups: &[Option<DiskGroup>], n: u8) -> AMResult<AMPointerGlobal> {
        let mut dg = diskgroups[n as usize].clone();

        let ent_each = (BLOCK_SIZE - std::mem::size_of::<LLGHeader>()) / std::mem::size_of::<T>();
        let blocks = if self.is_empty() {
            1
        } else {
            (self.len() + (ent_each - 1)) / ent_each
        };

        let mut blockptrs = (0..blocks)
            .map(|_| dg.as_mut().ok_or(AMErrorFS::NoDiskgroup)?.alloc_blocks(1))
            .collect::<AMResult<Vec<AMPointerGlobal>>>()?;
        blockptrs.push(AMPointerGlobal::null());
        let mut headers: Vec<LLGHeader> = (0..blocks)
            .map(|i| LLGHeader {
                count:    0,
                _padding: 0,
                next:     blockptrs[i + 1],
            })
            .collect();

        let mut it = self.iter();

        for i in 0..blocks {
            let mut buf = [0; BLOCK_SIZE];
            let mut pos = std::mem::size_of::<LLGHeader>();
            for _ in 0..ent_each {
                let next_pos = pos + std::mem::size_of::<T>();
                if let Some(v) = it.next() {
                    headers[i].count += 1;
                    unsafe {
                        buf[pos..next_pos].copy_from_slice(any_as_u8_slice(v));
                    }
                } else {
                    break;
                }
                pos = next_pos;
            }
            unsafe {
                buf[0..std::mem::size_of::<LLGHeader>()]
                    .copy_from_slice(any_as_u8_slice(&headers[i]));
            }
            blockptrs[i].write(0, BLOCK_SIZE, diskgroups, &buf)?;
        }
        for i in (0..blocks).rev() {
            if i == blocks - 1 {
                continue;
            }
            headers[i].next.update(diskgroups)?;
            let mut buf = [0; BLOCK_SIZE];
            blockptrs[i].read(0, BLOCK_SIZE, diskgroups, &mut buf)?;
            unsafe {
                buf[0..std::mem::size_of::<LLGHeader>()]
                    .copy_from_slice(any_as_u8_slice(&headers[i]));
            }
            blockptrs[i].write(0, BLOCK_SIZE, diskgroups, &buf)?;
        }

        blockptrs[0].update(diskgroups)?;

        Ok(blockptrs[0])
    }
    #[cfg(feature = "unstable")]
    fn prealloc(
        &self,
        count: usize,
        diskgroups: &mut [Option<DiskGroup>],
        n: u8,
    ) -> AMResult<Vec<AMPointerGlobal>> {
        let ent_each =
            (crate::BLOCK_SIZE - std::mem::size_of::<LLGHeader>()) / std::mem::size_of::<T>();
        let blocks = if count == 0 {
            1
        } else {
            (count + (ent_each - 1)) / ent_each
        };
        diskgroups[n as usize]
            .as_mut()
            .ok_or(AMErrorFS::NoDiskgroup)?
            .alloc_many(blocks as u64)
    }
    #[cfg(feature = "unstable")]
    fn write_preallocd(
        &self,
        diskgroups: &[Option<DiskGroup>],
        blocks: &[AMPointerGlobal],
    ) -> AMResult<AMPointerGlobal> {
        let mut blockptrs = blocks.to_vec();

        let ent_each = (BLOCK_SIZE - std::mem::size_of::<LLGHeader>()) / std::mem::size_of::<T>();
        let blocks = if self.is_empty() {
            1
        } else {
            (self.len() + (ent_each - 1)) / ent_each
        };

        assert_eq!(blockptrs.len(), blocks);
        blockptrs.push(AMPointerGlobal::null());
        let mut headers: Vec<LLGHeader> = (0..blocks)
            .map(|i| LLGHeader {
                count:    0,
                _padding: 0,
                next:     blockptrs[i + 1],
            })
            .collect();

        let mut it = self.iter();

        for i in 0..blocks {
            let mut buf = [0; BLOCK_SIZE];
            let mut pos = std::mem::size_of::<LLGHeader>();
            for _ in 0..ent_each {
                let next_pos = pos + std::mem::size_of::<T>();
                if let Some(v) = it.next() {
                    headers[i].count += 1;
                    unsafe {
                        buf[pos..next_pos].copy_from_slice(any_as_u8_slice(v));
                    }
                } else {
                    break;
                }
                pos = next_pos;
            }
            unsafe {
                buf[0..std::mem::size_of::<LLGHeader>()]
                    .copy_from_slice(any_as_u8_slice(&headers[i]));
            }
            blockptrs[i].write(0, BLOCK_SIZE, diskgroups, &buf)?;
        }
        for i in (0..blocks).rev() {
            if i == blocks - 1 {
                continue;
            }
            headers[i].next.update(diskgroups)?;
            let mut buf = [0; BLOCK_SIZE];
            blockptrs[i].read(0, BLOCK_SIZE, diskgroups, &mut buf)?;
            unsafe {
                buf[0..std::mem::size_of::<LLGHeader>()]
                    .copy_from_slice(any_as_u8_slice(&headers[i]));
            }
            blockptrs[i].write(0, BLOCK_SIZE, diskgroups, &buf)?;
        }

        blockptrs[0].update(diskgroups)?;

        Ok(blockptrs[0])
    }
}

#[test]
fn rw_test_global_empty() {
    #![allow(clippy::unwrap_used)]

    let dg = crate::test::dg::create_dg_mem_single(10000);

    let a: Vec<u32> = Vec::new();

    let ptr = LinkedListGlobal::write(&a, &vec![Some(dg.clone())], 0).unwrap();

    let a2 = <Vec<u32> as LinkedListGlobal<Vec<u32>>>::read(&vec![Some(dg)], ptr).unwrap();

    assert_eq!(a, a2);
}

#[test]
fn rw_test_global_base() {
    #![allow(clippy::unwrap_used)]

    let dg = crate::test::dg::create_dg_mem_single(10000);

    let mut a: Vec<u32> = Vec::new();

    for _ in 0..2000 {
        a.push(rand::random());
    }

    let ptr = LinkedListGlobal::write(&a, &vec![Some(dg.clone())], 0).unwrap();

    let a2 = <Vec<u32> as LinkedListGlobal<Vec<u32>>>::read(&vec![Some(dg)], ptr).unwrap();

    assert_eq!(a, a2);
}

#[test]
fn size_test_global() {
    use std::mem;
    assert_eq!(mem::size_of::<LLGHeader>(), 32);
}
