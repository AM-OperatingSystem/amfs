use crate::{AMPointerGlobal, DiskGroup, AMFS};

use crate::BLOCK_SIZE;

use amos_std::AMResult;

use std::convert::{TryFrom, TryInto};

use std::collections::{BTreeMap, VecDeque};

pub const LIST_HEADER_SIZE: usize = 16;
pub const FRAGMENT_SIZE: usize = 32;

/// An object set- the on-disk format to store the set of all objects.
#[derive(Clone, Debug)]
pub struct ObjectSet {
    pub(crate) ptr: AMPointerGlobal,
    dgs: [Option<DiskGroup>; 16],
}

/// Header for object list
pub struct ObjectListHeader {
    /// Index of first object in this list
    pub start_idx: u64,
    /// Number of objects in this list
    pub n_entries: u64,
}

impl ObjectListHeader {
    /// Create header from bytes
    #[cfg(feature = "stable")]
    pub fn from_bytes(buf: [u8; LIST_HEADER_SIZE]) -> Self {
        unsafe { std::ptr::read(buf.as_ptr() as *const _) }
    }
    /// Convert header to bytes
    #[cfg(feature = "stable")]
    pub fn to_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (self as *const Self) as *const u8,
                std::mem::size_of::<Self>(),
            )
        }
    }
}

impl ObjectSet {
    /// Creates a new object set handle
    #[cfg(feature = "stable")]
    pub fn read(dgs: [Option<DiskGroup>; 16], ptr: AMPointerGlobal) -> AMResult<ObjectSet> {
        Ok(ObjectSet { ptr, dgs })
    }
    /// Gets the object with a given ID
    #[cfg(feature = "stable")]
    pub(crate) fn get_object(&self, id: u64) -> AMResult<Option<Object>> {
        let mut to_process = VecDeque::new();
        to_process.push_back(self.ptr);
        loop {
            let ptr = to_process.pop_front();
            if ptr.is_none() {
                break;
            }
            let ptr = ptr.expect("PANIC");
            let blk = ptr.read_vec(&self.dgs)?;
            let header =
                ObjectListHeader::from_bytes(blk[..LIST_HEADER_SIZE].try_into().or(Err(0))?);
            if header.n_entries & 0x8000000000000000 != 0 {
                unimplemented!();
            } else {
                if header.start_idx <= id {
                    let mut pos = std::mem::size_of::<ObjectListHeader>();
                    let mut idx = header.start_idx;
                    while idx < id {
                        loop {
                            if u64::from_le_bytes(blk[pos..pos + 8].try_into().or(Err(0))?) == 0 {
                                pos += 8;
                                break;
                            }
                            pos += FRAGMENT_SIZE;
                            idx += 1;
                        }
                    }
                    let mut frags = Vec::new();
                    loop {
                        if u64::from_le_bytes(blk[pos..pos + 8].try_into().or(Err(0))?) == 0 {
                            break;
                        }
                        frags.push(Fragment::from_bytes(
                            blk[pos..pos + FRAGMENT_SIZE].try_into().or(Err(0))?,
                        ));
                        pos += FRAGMENT_SIZE;
                    }
                    return Ok(Some(Object { frags }));
                }
            }
        }
        Ok(None)
    }
    /// Gets all objects in the filesystem
    #[cfg(feature = "stable")]
    pub(crate) fn get_objects(&self) -> AMResult<BTreeMap<u64, Object>> {
        let mut res = BTreeMap::new();
        let mut to_process = VecDeque::new();
        to_process.push_back(self.ptr);
        loop {
            let ptr = to_process.pop_front();
            if ptr.is_none() {
                break;
            }
            let ptr = ptr.expect("PANIC");
            let blk = ptr.read_vec(&self.dgs)?;
            let header =
                ObjectListHeader::from_bytes(blk[..LIST_HEADER_SIZE].try_into().or(Err(0))?);
            let mut pos = std::mem::size_of::<ObjectListHeader>();
            let idx = header.start_idx;
            if header.n_entries & 0x8000000000000000 != 0 {
                unimplemented!();
            } else {
                for i in idx..idx + header.n_entries {
                    let mut frags = Vec::new();
                    loop {
                        if u64::from_le_bytes(blk[pos..pos + 8].try_into().or(Err(0))?) == 0 {
                            pos += 8;
                            break;
                        }
                        frags.push(Fragment::from_bytes(
                            blk[pos..pos + FRAGMENT_SIZE].try_into().or(Err(0))?,
                        ));
                        pos += FRAGMENT_SIZE;
                    }
                    res.insert(i, Object { frags });
                }
            }
        }
        Ok(res)
    }
    /// Updates or inserts an object
    #[cfg(feature = "unstable")]
    pub fn set_object(&self, fs: &mut AMFS, id: u64, obj: Object) -> AMResult<ObjectSet> {
        let mut res = self.clone();
        let mut to_process = VecDeque::new();
        to_process.push_back(self.ptr);
        let mut parents = vec![self.ptr];
        loop {
            let ptr = to_process.pop_front();
            if ptr.is_none() {
                break;
            }
            let ptr = ptr.expect("PANIC");
            let mut blk = ptr.read_vec(&self.dgs)?;
            let mut header =
                ObjectListHeader::from_bytes(blk[..LIST_HEADER_SIZE].try_into().or(Err(0))?);
            if header.n_entries & 0x8000000000000000 != 0 {
                unimplemented!();
            } else {
                if header.start_idx <= id {
                    let mut pos = LIST_HEADER_SIZE;
                    let mut idx = header.start_idx;
                    while idx < id {
                        loop {
                            if u64::from_le_bytes(blk[pos..pos + 8].try_into().or(Err(0))?) == 0 {
                                pos += 8;
                                break;
                            }
                            pos += FRAGMENT_SIZE;
                        }
                        idx += 1;
                    }
                    if id == header.start_idx + header.n_entries {
                        // We're appending an object
                        header.n_entries += 1;
                        let obj_size = FRAGMENT_SIZE * obj.frags.len() + 8;
                        if pos + obj_size < BLOCK_SIZE {
                            // No action needed, we're at the right spot
                        } else {
                            // We need to allocate a new block
                            unimplemented!();
                        }
                    } else {
                        // We're updating an object
                        assert_lt!(id, header.start_idx + header.n_entries);
                        let obj_size = std::mem::size_of::<Fragment>() * obj.frags.len() + 8;
                        let mut i = pos;
                        loop {
                            if u64::from_le_bytes(blk[i..i + 8].try_into().or(Err(0))?) == 0 {
                                i += 8;
                                break;
                            }
                            i += FRAGMENT_SIZE;
                        }
                        let slot_size = i - pos;
                        if obj_size == slot_size {
                            // No action needed, the new object is the same size
                        } else {
                            // We need to rearrange the list
                            unimplemented!();
                        }
                    }
                    for frag in &obj.frags {
                        blk[pos..pos + FRAGMENT_SIZE].copy_from_slice(frag.to_bytes());
                        pos += FRAGMENT_SIZE;
                    }
                    blk[pos..pos + 4].copy_from_slice(&[0u8; 4]);
                    blk[..LIST_HEADER_SIZE].copy_from_slice(header.to_bytes());

                    let mut ptr = fs.realloc(ptr)?.ok_or(0)?;
                    for _w in parents.windows(2) {
                        unimplemented!();
                    }
                    ptr.write(0, blk.len(), &self.dgs, &blk)?;
                    ptr.update(&self.dgs)?;
                    res.ptr = ptr;
                    break;
                } else {
                    println!(
                        "{}-{} {}",
                        header.start_idx,
                        header.start_idx + header.n_entries,
                        id
                    );
                }
            }
        }
        Ok(res)
    }
    /// Gets the size of an object
    pub fn size_object(&self, id: u64) -> AMResult<u64> {
        self.get_object(id)?.ok_or(0)?.size()
    }
    /// Reads the contents of an object
    pub fn read_object(
        &self,
        id: u64,
        start: u64,
        data: &mut [u8],
        dgs: &[Option<DiskGroup>],
    ) -> AMResult<u64> {
        self.get_object(id)?.ok_or(0)?.read(start, data, dgs)
    }
}

/// Represents one file or meta-file on disk
#[derive(Debug, PartialEq)]
pub struct Object {
    frags: Vec<Fragment>,
}

impl Object {
    /// Create a new object from a list of fragments
    pub fn new(frags: &[Fragment]) -> Object {
        Object {
            frags: frags.to_vec(),
        }
    }
    /// Reads the contents of an object from the disk
    #[cfg(feature = "stable")]
    fn read(&self, start: u64, data: &mut [u8], dgs: &[Option<DiskGroup>]) -> AMResult<u64> {
        let mut res = 0;
        let mut pos = 0;
        for f in &self.frags {
            if start < pos + f.size {
                let slice_start = start - pos;
                let slice_end = slice_start + u64::try_from(data.len())?;
                if slice_end > f.size {
                    unimplemented!();
                } else {
                    res += f
                        .pointer
                        .read(slice_start.try_into()?, data.len(), dgs, data)?;
                }
            }
            pos += f.size;
        }
        Ok(res.try_into()?)
    }
    /// Writes the contents of an object to the disk
    #[cfg(feature = "unstable")]
    pub(crate) fn write(
        &mut self,
        handle: &mut AMFS,
        start: u64,
        data: &[u8],
        dgs: &[Option<DiskGroup>],
    ) -> AMResult<u64> {
        let mut res = 0;
        let mut pos = 0;
        for f in &mut self.frags {
            if start < pos + f.size {
                let slice_start = start - pos;
                let slice_end = slice_start + u64::try_from(data.len())?;
                if slice_end > f.size {
                    unimplemented!();
                } else {
                    f.pointer = handle.realloc(f.pointer)?.ok_or(0)?;
                    res += f
                        .pointer
                        .write(slice_start.try_into()?, data.len(), dgs, data)?;
                    f.pointer.update(dgs)?;
                }
            }
            pos += f.size;
        }
        Ok(res.try_into()?)
    }
    /// Fetches the size of the object
    #[cfg(feature = "stable")]
    fn size(self) -> AMResult<u64> {
        let mut res = 0;
        for f in self.frags {
            res += f.size;
        }
        Ok(res)
    }
}

/// A single contiguous fragment of a file
#[derive(Debug, PartialEq, Clone)]
#[repr(C)]
pub struct Fragment {
    size: u64,
    offset: u64,
    pointer: AMPointerGlobal,
}

impl Fragment {
    /// Creates a new fragment
    pub fn new(size: u64, offset: u64, pointer: AMPointerGlobal) -> Fragment {
        Fragment {
            size,
            offset,
            pointer,
        }
    }
    /// Initializes a fragment from a slice of bytes
    #[cfg(feature = "stable")]
    pub fn from_bytes(buf: [u8; FRAGMENT_SIZE]) -> Fragment {
        unsafe { std::ptr::read(buf.as_ptr() as *const _) }
    }
    /// Converts a fragment to a slice of bytes
    #[cfg(feature = "stable")]
    pub fn to_bytes(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                (self as *const Self) as *const u8,
                std::mem::size_of::<Self>(),
            )
        }
    }
}

#[test]
fn list_header_size_test() {
    use std::mem;
    assert_eq!(mem::size_of::<ObjectListHeader>(), LIST_HEADER_SIZE);
}

#[test]
fn list_fragment_size_test() {
    use std::mem;
    assert_eq!(mem::size_of::<Fragment>(), FRAGMENT_SIZE);
}

#[test]
#[serial]
#[allow(clippy::unwrap_used)]
pub fn test_object() {
    crate::test::logging::init_log();

    let fs = crate::test::fsinit::create_fs().unwrap();

    let mut buf = [0u8; 4];
    assert_eq!(fs.read_object(0, 0, &mut buf).unwrap(), 0);
}

#[test]
#[serial]
#[allow(clippy::unwrap_used)]
pub fn test_insert() {
    crate::test::logging::init_log();

    let fs = crate::test::fsinit::create_fs().unwrap();

    fs.create_object(0, 1).unwrap();
    fs.create_object(1, 2).unwrap();
    fs.create_object(2, 3).unwrap();
    fs.create_object(3, 4).unwrap();
    fs.sync().unwrap();
    assert_eq!(fs.size_object(0).unwrap(), 1);
    assert_eq!(fs.size_object(1).unwrap(), 2);
    assert_eq!(fs.size_object(2).unwrap(), 3);
    assert_eq!(fs.size_object(3).unwrap(), 4);
    assert_eq!(fs.write_object(0, 0, &[0]).unwrap(), 1);
    assert_eq!(fs.write_object(1, 0, &[0, 1]).unwrap(), 2);
    assert_eq!(fs.write_object(2, 0, &[0, 1, 2]).unwrap(), 3);
    assert_eq!(fs.write_object(3, 0, &[0, 1, 2, 3]).unwrap(), 4);
    let mut buf = [0u8; 4];
    assert_eq!(fs.read_object(0, 0, &mut buf[0..1]).unwrap(), 1);
    assert_eq!(buf, [0u8, 0u8, 0u8, 0u8]);
    assert_eq!(fs.read_object(1, 0, &mut buf[0..2]).unwrap(), 2);
    assert_eq!(buf, [0u8, 1u8, 0u8, 0u8]);
    assert_eq!(fs.read_object(2, 0, &mut buf[0..3]).unwrap(), 3);
    assert_eq!(buf, [0u8, 1u8, 2u8, 0u8]);
    assert_eq!(fs.read_object(3, 0, &mut buf[0..4]).unwrap(), 4);
    assert_eq!(buf, [0u8, 1u8, 2u8, 3u8]);
    fs.commit().unwrap();
}
