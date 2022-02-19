use std::{
    collections::{BTreeMap, VecDeque},
    convert::{TryFrom, TryInto},
};

use amos_std::AMResult;

use crate::{AMPointerGlobal, DiskGroup, AMFS, BLOCK_SIZE};

pub const LIST_HEADER_SIZE: usize = 16;
pub const FRAGMENT_SIZE: usize = 32;

/// An object set- the on-disk format to store the set of all objects.
#[derive(Clone, Debug)]
pub struct ObjectSet {
    pub(crate) ptr: AMPointerGlobal,
    dgs:            Vec<Option<DiskGroup>>,
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
    pub fn read(dgs: Vec<Option<DiskGroup>>, ptr: AMPointerGlobal) -> ObjectSet {
        ObjectSet { ptr, dgs }
    }
    /// Checks the existance of an object with a given ID
    #[cfg(feature = "stable")]
    pub fn exists_object(&self, id: u64) -> AMResult<bool> {
        Ok(self.get_object(id)?.is_some())
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
                todo!();
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
                todo!();
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
        let parents = vec![self.ptr];
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
                //If the high bit is set, this is an indirect block.
                todo!();
            } else {
                if header.start_idx <= id {
                    //We're in the block containing the object to update
                    let mut pos = LIST_HEADER_SIZE;
                    let mut idx = header.start_idx;
                    while idx < id {
                        //Scan forward until we're at the start of the object to update
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
                            todo!();
                        }
                    } else {
                        // We're updating an object
                        assert_lt!(id, header.start_idx + header.n_entries);
                        // Calculate the size of the new object
                        let obj_size = std::mem::size_of::<Fragment>() * obj.frags.len() + 8;
                        let mut i = pos;
                        // Scan forward to the end of the old object
                        loop {
                            if u64::from_le_bytes(blk[i..i + 8].try_into().or(Err(0))?) == 0 {
                                i += 8;
                                break;
                            }
                            i += FRAGMENT_SIZE;
                        }
                        idx += 1;
                        // Calculate the size used by the old object
                        let slot_size = i - pos;
                        // Check if the new object is the same size as the old
                        if obj_size == slot_size {
                            // No action needed, the new object is the same size
                        } else {
                            let size_diff = obj_size - slot_size;
                            let mut j = i;
                            // Scan forward to the end of the last object in the block
                            while idx < (header.start_idx + header.n_entries) - 1 {
                                loop {
                                    if u64::from_le_bytes(blk[j..j + 8].try_into().or(Err(0))?) == 0
                                    {
                                        j += 8;
                                        break;
                                    }
                                    j += FRAGMENT_SIZE;
                                }
                                idx += 1;
                            }
                            // Calculate the new end of the last object after shifting
                            let new_end = j + size_diff;
                            if new_end > BLOCK_SIZE {
                                // We need to spill into a new block
                                todo!();
                            } else {
                                blk.copy_within(i..j, i + size_diff);
                            }
                            /*println!(
                                "i:{} si:{} ne:{} p:{} i:{} j:{} sd:{} nl:{}",
                                idx,
                                header.start_idx,
                                header.n_entries,
                                pos,
                                i,
                                j,
                                size_diff,
                                i + size_diff
                            );*/
                            //todo!();
                        }
                    }
                    //println!("{}", pos);
                    for frag in &obj.frags {
                        blk[pos..pos + FRAGMENT_SIZE].copy_from_slice(frag.to_bytes());
                        pos += FRAGMENT_SIZE;
                    }
                    blk[pos..pos + 8].copy_from_slice(&[0u8; 8]);

                    //pos += 8;
                    //println!("{}", pos);

                    blk[..LIST_HEADER_SIZE].copy_from_slice(header.to_bytes());

                    let mut ptr = fs.realloc(ptr)?.ok_or(0)?;
                    for _w in parents.windows(2) {
                        todo!();
                    }
                    ptr.write(0, blk.len(), &self.dgs, &blk)?;
                    ptr.update(&self.dgs)?;
                    res.ptr = ptr;
                    return Ok(res);
                } else {
                    //We're not in the right block, keep searching
                    println!(
                        "{}-{} {}",
                        header.start_idx,
                        header.start_idx + header.n_entries,
                        id
                    );
                    todo!();
                }
            }
        }
        panic!();
    }
    /// Gets the size of an object
    #[cfg(feature = "stable")]
    pub fn size_object(&self, id: u64) -> AMResult<u64> {
        self.get_object(id)?.ok_or(0)?.size()
    }
    /// Reads the contents of an object
    #[cfg(feature = "stable")]
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
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Object {
    frags: Vec<Fragment>,
}

impl Object {
    /// Create a new object from a list of fragments
    #[cfg(feature = "stable")]
    pub fn new(frags: &[Fragment]) -> Object {
        Object {
            frags: frags.to_vec(),
        }
    }
    /// Return the list of fragments backing the object
    #[cfg(feature = "unstable")]
    pub fn frags(&self) -> Vec<Fragment> {
        self.frags.clone()
    }
    /// Reads the contents of an object from the disk
    #[cfg(feature = "unstable")]
    fn read(&self, start: u64, data: &mut [u8], dgs: &[Option<DiskGroup>]) -> AMResult<u64> {
        let mut res = 0;
        let mut frag_start = 0;
        let end = start + u64::try_from(data.len())?;
        for f in &self.frags {
            let frag_end = frag_start + f.size;
            if frag_start >= end {
                break;
            }
            if frag_end > start {
                let frag_read_start = if frag_start < start {
                    start - frag_start
                } else {
                    0
                };
                let buf_read_start = if frag_start < start {
                    0
                } else {
                    frag_start - start
                }
                .try_into()?;
                let read_len = if frag_start < start && frag_end > end {
                    end - start
                } else if frag_start < start {
                    frag_end - start
                } else if frag_end > end {
                    end - frag_start
                } else {
                    f.size
                }
                .try_into()?;
                res += f.pointer.read(
                    frag_read_start.try_into()?,
                    read_len,
                    dgs,
                    &mut data[buf_read_start..buf_read_start + read_len],
                )?;
            }
            frag_start = frag_end;
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
                    todo!();
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
    #[cfg(feature = "unstable")]
    pub(crate) fn truncate(
        &mut self,
        handle: &mut AMFS,
        size: u64,
        _dgs: &[Option<DiskGroup>],
    ) -> AMResult<()> {
        if self.frags.is_empty() {
            if size == 0 {
                // No-op
            } else {
                //We need to create fragments
                todo!();
            }
        } else {
            let mut cur_size = self.size()?;
            if size < cur_size {
                // We want to shrink
                while let Some(lf) = self.frags.last_mut() {
                    if cur_size - lf.size > size {
                        // Dropping a fragment leaves us too big, continue
                        cur_size -= lf.size;
                        self.frags.pop();
                        //TODO: Free the fragment
                    } else if cur_size - lf.size == size {
                        // Dropping a fragment leaves us the right size
                        self.frags.pop();
                        //TODO: Free the fragment
                        break;
                    } else {
                        // Shrinking a fragment leaves us the right size
                        lf.size = cur_size - size;
                        break;
                    }
                }
            } else {
                let mut new_frags = handle.alloc_bytes(size - self.size()?)?;
                self.frags.append(&mut new_frags);
            }
        }
        Ok(())
    }
    /// Fetches the size of the object
    #[cfg(feature = "stable")]
    fn size(&self) -> AMResult<u64> {
        let mut res = 0;
        for f in &self.frags {
            res += f.size;
        }
        Ok(res)
    }
}

/// A single contiguous fragment of a file
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
#[repr(C)]
pub struct Fragment {
    /// The length of the fragment, in bytes
    pub size:    u64,
    /// The offset from the pointer location to the start of the fragment
    pub offset:  u64,
    /// A pointer to the block containing the fragment's data
    pub pointer: AMPointerGlobal,
}

impl Fragment {
    /// Creates a new fragment
    #[cfg(feature = "stable")]
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
#[allow(clippy::unwrap_used)]
pub fn test_object() {
    crate::test::logging::init_log();

    let fs = crate::test::fsinit::create_fs().unwrap();

    let mut buf = [0u8; 4];
    assert_eq!(fs.read_object(0, 0, &mut buf).unwrap(), 0);
}

#[test]
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

#[test]
#[allow(clippy::unwrap_used)]
pub fn test_truncate() {
    crate::test::logging::init_log();

    let fs = crate::test::fsinit::create_fs().unwrap();

    fs.create_object(0, 8).unwrap();
    fs.create_object(1, 1).unwrap();
    fs.create_object(2, 1).unwrap();
    fs.create_object(3, 1).unwrap();
    fs.sync().unwrap();
    assert_eq!(fs.size_object(0).unwrap(), 8);
    assert_eq!(fs.write_object(0, 0, &[0, 1, 2, 3, 4, 5, 6, 7]).unwrap(), 8);
    let mut buf = [0u8; 8];
    assert_eq!(fs.read_object(0, 0, &mut buf[0..8]).unwrap(), 8);
    assert_eq!(buf, [0u8, 1u8, 2u8, 3u8, 4u8, 5u8, 6u8, 7u8]);
    fs.truncate_object(0, 4).unwrap();
    assert_eq!(fs.size_object(0).unwrap(), 4);
    let mut buf = [0u8; 4];
    assert_eq!(fs.read_object(0, 0, &mut buf[0..4]).unwrap(), 4);
    assert_eq!(buf, [0u8, 1u8, 2u8, 3u8]);
    fs.commit().unwrap();
    fs.truncate_object(0, 16).unwrap();
    assert_eq!(fs.size_object(0).unwrap(), 16);
    let mut buf = [0u8; 16];
    assert_eq!(fs.read_object(0, 0, &mut buf[0..16]).unwrap(), 16);
    fs.commit().unwrap();
}
