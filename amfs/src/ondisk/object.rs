
use crate::{AMPointerGlobal,DiskGroup};
use amos_std::AMResult;

use std::convert::TryInto;

use std::collections::BTreeMap;

/// An object set- the on-disk format to store the set of all objects.
pub struct ObjectSet {
    ptr: AMPointerGlobal,
    dgs: [Option<DiskGroup>;16],
}

pub struct ObjectListHeader {
    next: AMPointerGlobal,
    start_idx: u64,
    n_entries: u8,
}

impl ObjectListHeader {
    pub fn from_bytes(buf: [u8;24]) -> ObjectListHeader {
        unsafe { std::ptr::read(buf.as_ptr() as *const _) }
    }
}

impl ObjectSet {
    /// Creates a new object set handle
    pub fn new(ptr: AMPointerGlobal, dgs: [Option<DiskGroup>;16]) -> AMResult<ObjectSet> {
        Ok(ObjectSet{ptr,dgs})
    }
    /// Gets the object with a given ID
    pub fn get_object(&self, id: u64) -> AMResult<Option<Object>> {
        let mut ptr = self.ptr;
        loop {
            if ptr.is_null() { break; }
            let blk = ptr.read_vec(&self.dgs)?;
            let header = ObjectListHeader::from_bytes(blk[..24].try_into().or(Err(0))?);
            ptr = header.next;
            if header.start_idx<=id {
                let mut pos = 24;
                let mut idx = header.start_idx;
                while idx < id {
                    loop {
                        if u32::from_le_bytes(blk[pos..pos+4].try_into().or(Err(0))?) == 0 {
                            break;
                        }
                        pos+=24;
                        idx+=1;
                    }
                }
                let mut frags = Vec::new();
                loop {
                    if u32::from_le_bytes(blk[pos..pos+4].try_into().or(Err(0))?) == 0 {
                        break;
                    }
                    frags.push(Fragment::from_bytes(blk[pos..pos+24].try_into().or(Err(0))?));
                    pos+=24;
                }
                return Ok(Some(Object{frags}));
            }
        }
        Ok(None)
    }
    /// Gets all objects in the filesystem
    pub fn get_objects(&self) -> AMResult<BTreeMap<u64,Object>> {
        let mut res = BTreeMap::new();
        let mut ptr = self.ptr;
        loop {
            if ptr.is_null() { break; }
            let blk = ptr.read_vec(&self.dgs)?;
            let header = ObjectListHeader::from_bytes(blk[..24].try_into().or(Err(0))?);
            ptr = header.next;
            let mut pos = std::mem::size_of::<ObjectListHeader>();
            let idx = header.start_idx;
            for i in idx..idx+u64::from(header.n_entries) {
                let mut frags = Vec::new();
                loop {
                    if u32::from_le_bytes(blk[pos..pos+4].try_into().or(Err(0))?) == 0 {
                        break;
                    }
                    frags.push(Fragment::from_bytes(blk[pos..pos+24].try_into().or(Err(0))?));
                    pos+=24;
                }
                res.insert(i,Object{frags});
            }
        }
        Ok(res)
    }
}

/// Represents one file or meta-file on disk
#[derive(Debug,PartialEq)]
pub struct Object {
    frags: Vec<Fragment>,
}

impl Object {
    /// Reads the contents of an object from the disk
    pub fn read(self) -> AMResult<Vec<u8>> {
        let res = Vec::new();
        for _ in self.frags {
            unimplemented!();
        }
        Ok(res)
    }
}

#[derive(Debug,PartialEq)]
#[repr(C)]
pub struct Fragment {    
    size: u64,
    offset: u16,
    pointer: AMPointerGlobal,
}

impl Fragment {
    pub fn from_bytes(buf: [u8;32]) -> Fragment {
        unsafe { std::ptr::read(buf.as_ptr() as *const _) }
    }
}

#[test]
fn list_header_size_test() {
    use std::mem;
    assert_eq!(mem::size_of::<ObjectListHeader>(), 32);
}

#[test]
#[serial]
#[allow(clippy::unwrap_used)]
pub fn test_object() {
    crate::test::logging::init_log();

    let fs = crate::test::fsinit::create_fs().unwrap();

    assert_eq!(fs.read_object(0).unwrap().len(),0);
}