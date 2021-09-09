use crate::{AMPointerGlobal,DiskGroup};
use amos_std::AMResult;
use crate::BLOCK_SIZE;

unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts(
        (p as *const T) as *const u8,
        ::std::mem::size_of::<T>(),
    )
}

unsafe fn u8_slice_as_any<T: Sized>(p: &[u8]) -> &T {
    assert!(p.len()>=::std::mem::size_of::<T>());
    &*((p.as_ptr() as *const u8) as *const T)
}

#[repr(C)]
pub(crate) struct LLGHeader {
    next: AMPointerGlobal,
    count: u16,
}

/// Trait for writing a collection of items to disk, using global pointers.
pub trait LinkedListGlobal<T: Sized> {
    /// Reads the linked list from disk
    fn read(d: &[Option<DiskGroup>], p: AMPointerGlobal) -> AMResult<T>;
    /// Writes the linked list to disk
    fn write(&self, d: &[Option<DiskGroup>], n: u8) -> AMResult<AMPointerGlobal>;
    /// Writes the linked list to disk, using previously allocated blocks
    fn prealloc(&self, count: usize, d: &mut [Option<DiskGroup>], n:u8) -> AMResult<Vec<AMPointerGlobal>>;
    /// Writes the linked list to disk, using previously allocated blocks
    fn write_preallocd(&self, d: &[Option<DiskGroup>], blocks: &[AMPointerGlobal]) -> AMResult<AMPointerGlobal>;
}

impl<T: Copy> LinkedListGlobal<Vec<T>> for Vec<T> {
    fn read(dgs: &[Option<DiskGroup>], mut p: AMPointerGlobal) -> AMResult<Vec<T>> {
        let mut res = Vec::new();
        let mut buf = [0;BLOCK_SIZE];
        loop {
            if p.is_null() { break; }
            let count;
            assert!(p.validate(dgs)?);
            p.read(0,BLOCK_SIZE,dgs,&mut buf)?;
            unsafe {
                let hdr = u8_slice_as_any::<LLGHeader>(&buf);
                p=hdr.next;
                count = hdr.count;
            }
            for i in 0..(count as usize) {
                unsafe {
                    let addr = std::mem::size_of::<LLGHeader>() + std::mem::size_of::<T>()*i;
                    let ent = u8_slice_as_any::<T>(&buf[addr..]);
                    res.push(*ent);
                }
            }
        }
        Ok(res)
    }
    fn write(&self, dgs: &[Option<DiskGroup>], n: u8) -> AMResult<AMPointerGlobal> {
        let mut dg = dgs[n as usize].clone();

        let ent_each = (BLOCK_SIZE - std::mem::size_of::<LLGHeader>())/std::mem::size_of::<T>();
        let blks = if self.is_empty() { 1 } else { (self.len()+(ent_each-1))/ent_each };
        
        let mut blockptrs = (0..blks).map(|_| dg.as_mut().ok_or(0)?.alloc(1) ).collect::<AMResult<Vec<AMPointerGlobal>>>()?;
        blockptrs.push(AMPointerGlobal::null());
        let mut headers : Vec<LLGHeader> = (0..blks).map(|i| LLGHeader{count:0,next:blockptrs[i+1]}).collect();

        let mut it = self.iter();

        for i in 0..blks {
            let mut buf = [0;BLOCK_SIZE];
            let mut pos = std::mem::size_of::<LLGHeader>();
            for _ in 0..ent_each {
                let npos = pos + std::mem::size_of::<T>();
                if let Some(v) = it.next() {
                    headers[i].count+=1;
                    unsafe{
                        buf[pos..npos].copy_from_slice(any_as_u8_slice(v));
                    }
                } else {
                    break;
                }
                pos=npos;
            }
            unsafe{
                buf[0..std::mem::size_of::<LLGHeader>()].copy_from_slice(any_as_u8_slice(&headers[i]));
            }
            blockptrs[i].write(0,BLOCK_SIZE,dgs, &buf)?;
        }
        for i in (0..blks).rev() {
                if i == blks-1 {
                    continue;
                }
                headers[i].next.update(dgs)?;
                let mut buf = [0;BLOCK_SIZE];
                blockptrs[i].read(0,BLOCK_SIZE,dgs, &mut buf)?;
                unsafe {
                    buf[0..std::mem::size_of::<LLGHeader>()].copy_from_slice(any_as_u8_slice(&headers[i]));
                }
                blockptrs[i].write(0,BLOCK_SIZE,dgs, &buf)?;
        }

        blockptrs[0].update(dgs)?;

        Ok(blockptrs[0])
    }
    fn prealloc(&self, count: usize, dgs: &mut [Option<DiskGroup>], n:u8 ) -> AMResult<Vec<AMPointerGlobal>> {
        let ent_each = (crate::BLOCK_SIZE - std::mem::size_of::<LLGHeader>())/std::mem::size_of::<T>();
        let blks = if count==0 { 1 } else { (count+(ent_each-1))/ent_each };
        dgs[n as usize].as_mut().ok_or(0)?.alloc_many(blks as u64)
    }
    fn write_preallocd(&self, dgs: &[Option<DiskGroup>], blks: &[AMPointerGlobal]) -> AMResult<AMPointerGlobal> {
        
        let mut blockptrs = blks.to_vec();

        let ent_each = (BLOCK_SIZE - std::mem::size_of::<LLGHeader>())/std::mem::size_of::<T>();
        let blks = if self.is_empty() { 1 } else { (self.len()+(ent_each-1))/ent_each };
        
        assert_eq!(blockptrs.len(),blks);
        blockptrs.push(AMPointerGlobal::null());
        let mut headers : Vec<LLGHeader> = (0..blks).map(|i| LLGHeader{count:0,next:blockptrs[i+1]}).collect();

        let mut it = self.iter();

        for i in 0..blks {
            let mut buf = [0;BLOCK_SIZE];
            let mut pos = std::mem::size_of::<LLGHeader>();
            for _ in 0..ent_each {
                let npos = pos + std::mem::size_of::<T>();
                if let Some(v) = it.next() {
                    headers[i].count+=1;
                    unsafe{
                        buf[pos..npos].copy_from_slice(any_as_u8_slice(v));
                    }
                } else {
                    break;
                }
                pos=npos;
            }
            unsafe{
                buf[0..std::mem::size_of::<LLGHeader>()].copy_from_slice(any_as_u8_slice(&headers[i]));
            }
            blockptrs[i].write(0,BLOCK_SIZE,dgs, &buf)?;
        }
        for i in (0..blks).rev() {
                if i == blks-1 {
                    continue;
                }
                headers[i].next.update(dgs)?;
                let mut buf = [0;BLOCK_SIZE];
                blockptrs[i].read(0,BLOCK_SIZE,dgs, &mut buf)?;
                unsafe {
                    buf[0..std::mem::size_of::<LLGHeader>()].copy_from_slice(any_as_u8_slice(&headers[i]));
                }
                blockptrs[i].write(0,BLOCK_SIZE,dgs, &buf)?;
        }

        blockptrs[0].update(dgs)?;

        Ok(blockptrs[0])
    }
}


#[test]
fn rw_test_global_empty() {
    #![allow(clippy::unwrap_used)]

    let dg = crate::test::dg::create_dg_mem_single(10000);

    let a : Vec<u32> = Vec::new();

    let ptr = LinkedListGlobal::write(&a, &vec![Some(dg.clone())],0).unwrap();

    let a2 = <Vec<u32> as LinkedListGlobal<Vec<u32>>>::read(&vec![Some(dg)],ptr).unwrap();

    assert_eq!(a,a2);
}

#[test]
fn rw_test_global_base() {
    #![allow(clippy::unwrap_used)]

    let dg = crate::test::dg::create_dg_mem_single(10000);

    let mut a : Vec<u32> = Vec::new();

    for _ in 0..2000 {
        a.push(rand::random());
    }

    let ptr = LinkedListGlobal::write(&a, &vec![Some(dg.clone())],0).unwrap();

    let a2 = <Vec<u32> as LinkedListGlobal<Vec<u32>>>::read(&vec![Some(dg)],ptr).unwrap();

    assert_eq!(a,a2);
}