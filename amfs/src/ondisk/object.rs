
use crate::{AMPointerGlobal,DiskGroup,AMFS};

use crate::BLOCK_SIZE;

use amos_std::AMResult;

use std::convert::{TryInto,TryFrom};

use std::collections::BTreeMap;

/// An object set- the on-disk format to store the set of all objects.
#[derive(Clone,Debug)]
pub struct ObjectSet {
    ptr: AMPointerGlobal,
    dgs: [Option<DiskGroup>;16],
}

/// Header for object list
pub struct ObjectListHeader {
    /// Next block in object list
    pub next: AMPointerGlobal,
    /// Index of first object in this list
    pub start_idx: u64,
    /// Number of objects in this list
    pub n_entries: u64,
}

impl ObjectListHeader {
    /// Create header from bytes
    #[cfg(feature="stable")]
    pub fn from_bytes(buf: [u8;32]) -> Self {
        unsafe { std::ptr::read(buf.as_ptr() as *const _) }
    }
    /// Convert header to bytes
    #[cfg(feature="stable")]
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
    #[cfg(feature="stable")]
    pub fn read(dgs: [Option<DiskGroup>;16],ptr: AMPointerGlobal) -> AMResult<ObjectSet> {
        Ok(ObjectSet{ptr,dgs})
    }
    /// Gets the object with a given ID
    #[cfg(feature="stable")]
    pub(crate) fn get_object(&self, id: u64) -> AMResult<Option<Object>> {
        let mut ptr = self.ptr;
        loop {
            if ptr.is_null() { break; }
            let blk = ptr.read_vec(&self.dgs)?;
            let header = ObjectListHeader::from_bytes(blk[..32].try_into().or(Err(0))?);
            ptr = header.next;
            if header.start_idx<=id {
                let mut pos = 32;
                let mut idx = header.start_idx;
                while idx < id {
                    loop {
                        if u64::from_le_bytes(blk[pos..pos+8].try_into().or(Err(0))?) == 0 {
                            pos+=8;
                            break;
                        }
                        pos+=32;
                        idx+=1;
                    }
                }
                let mut frags = Vec::new();
                loop {
                    if u64::from_le_bytes(blk[pos..pos+8].try_into().or(Err(0))?) == 0 {
                        break;
                    }
                    frags.push(Fragment::from_bytes(blk[pos..pos+32].try_into().or(Err(0))?));
                    pos+=32;
                }
                return Ok(Some(Object{frags}));
            }
        }
        Ok(None)
    }
    /// Gets all objects in the filesystem
    #[cfg(feature="stable")]
    pub(crate) fn get_objects(&self) -> AMResult<BTreeMap<u64,Object>> {
        let mut res = BTreeMap::new();
        let mut ptr = self.ptr;
        loop {
            if ptr.is_null() { break; }
            let blk = ptr.read_vec(&self.dgs)?;
            let header = ObjectListHeader::from_bytes(blk[..32].try_into().or(Err(0))?);
            ptr = header.next;
            let mut pos = std::mem::size_of::<ObjectListHeader>();
            let idx = header.start_idx;
            for i in idx..idx+header.n_entries {
                let mut frags = Vec::new();
                loop {
                    if u64::from_le_bytes(blk[pos..pos+8].try_into().or(Err(0))?) == 0 {
                        pos+=8;
                        break;
                    }
                    frags.push(Fragment::from_bytes(blk[pos..pos+32].try_into().or(Err(0))?));
                    pos+=32;
                }
                res.insert(i,Object{frags});
            }
        }
        Ok(res)
    }
    /// Updates or inserts an object
    #[cfg(feature="unstable")]
    pub fn set_object(&mut self, id: u64, obj: Object) -> AMResult<()>{
        let mut _ptr_prev = AMPointerGlobal::null();
        let mut ptr = self.ptr;
        if ptr.is_null() {
            unimplemented!();
        }
        loop {
            if ptr.is_null() { return Err(0.into()) }
            let mut blk = ptr.read_vec(&self.dgs)?;
            let mut header = ObjectListHeader::from_bytes(blk[..32].try_into().or(Err(0))?);
            if header.start_idx<=id {
                let mut pos = 32;
                let mut idx = header.start_idx;
                while idx < id {
                    loop {
                        if u64::from_le_bytes(blk[pos..pos+8].try_into().or(Err(0))?) == 0 {
                            pos+=8;
                            break;
                        }
                        pos+=32;
                    }
                    idx+=1;
                }
                if id == header.start_idx+header.n_entries { // We're appending an object
                    header.n_entries+=1;
                    let obj_size = std::mem::size_of::<Fragment>()*obj.frags.len() + 8;
                    if pos + obj_size < BLOCK_SIZE {
                        // No action needed, we're at the right spot
                    } else {
                        // We need to allocate a new block
                        unimplemented!();
                    }
                } else { // We're updating an object
                    assert_lt!(id,header.start_idx+header.n_entries);
                    let obj_size = std::mem::size_of::<Fragment>()*obj.frags.len() + 8;
                    let mut i = pos;
                    loop {
                        if u64::from_le_bytes(blk[i..i+8].try_into().or(Err(0))?) == 0 {
                            i+=8;
                            break;
                        }
                        i+=32;
                    }
                    let slot_size = i-pos;
                    if obj_size == slot_size {
                        // No action needed, the new object is the same size
                    } else {
                        // We need to rearrange the list
                        unimplemented!();
                    }
                }
                for frag in &obj.frags {
                    blk[pos..pos+32].copy_from_slice(frag.to_bytes());
                    pos+=32;
                }
                blk[pos..pos+4].copy_from_slice(&[0u8;4]);
                blk[..32].copy_from_slice(header.to_bytes());
                ptr.write(0, blk.len(), &self.dgs, &blk)?;
                ptr.update(&self.dgs)?;
                break;
            } else {
                println!("{}-{} {}",header.start_idx,header.start_idx+header.n_entries,id);
            }
            _ptr_prev = ptr;
            ptr = header.next;
        }
        Ok(())
    }
    /// Gets the size of an object
    pub fn size_object(&self, id: u64) -> AMResult<u64> {
        self.get_object(id)?.ok_or(0)?.size()
    }
    /// Reads the contents of an object
    pub fn read_object(&self, id: u64,start:u64,data:&mut [u8],dgs:&[Option<DiskGroup>]) -> AMResult<u64> {
        self.get_object(id)?.ok_or(0)?.read(start,data,dgs)
    }
}

/// Represents one file or meta-file on disk
#[derive(Debug,PartialEq)]
pub struct Object {
    frags: Vec<Fragment>,
}

impl Object {
    /// Reads the contents of an object from the disk
    #[cfg(feature="stable")]
    fn read(&self,start:u64,data:&mut [u8],dgs:&[Option<DiskGroup>]) -> AMResult<u64> {
        let mut res = 0;
        let mut pos = 0;
        for f in &self.frags {
            if start < pos+f.size {
                let slice_start = start-pos;
                let slice_end = slice_start+u64::try_from(data.len())?;
                if slice_end>f.size {
                    unimplemented!();
                } else {
                    res+=f.pointer.read(slice_start.try_into()?, data.len(), dgs, data)?;
                }
            }
            pos+=f.size;
        }
        Ok(res.try_into()?)
    }
    /// Writes the contents of an object to the disk
    #[cfg(feature="unstable")]
    pub(crate) fn write(&mut self,handle: &mut AMFS,start:u64,data:&[u8],dgs:&[Option<DiskGroup>]) -> AMResult<u64> {
        let mut res = 0;
        let mut pos = 0;
        for f in &mut self.frags {
            if start < pos+f.size {
                let slice_start = start-pos;
                let slice_end = slice_start+u64::try_from(data.len())?;
                if slice_end>f.size {
                    unimplemented!();
                } else {
                    f.pointer = handle.realloc(f.pointer)?.ok_or(0)?;
                    res+=f.pointer.write(slice_start.try_into()?, data.len(), dgs, data)?;
                    f.pointer.update(dgs)?;
                }
            }
            pos+=f.size;
        }
        Ok(res.try_into()?)
    }
    /// Fetches the size of the object
    #[cfg(feature="stable")]
    fn size(self) -> AMResult<u64> {
        let mut res = 0;
        for f in self.frags {
            res+=f.size;
        }
        Ok(res)
    }
}

#[derive(Debug,PartialEq)]
#[repr(C)]
pub struct Fragment {    
    size: u64,
    offset: u64,
    pointer: AMPointerGlobal,
}

impl Fragment {
    #[cfg(feature="stable")]
    pub fn from_bytes(buf: [u8;32]) -> Fragment {
        unsafe { std::ptr::read(buf.as_ptr() as *const _) }
    }
    #[cfg(feature="stable")]
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
    assert_eq!(mem::size_of::<ObjectListHeader>(), 32);
}

#[test]
fn list_fragment_size_test() {
    use std::mem;
    assert_eq!(mem::size_of::<Fragment>(), 32);
}

#[test]
#[serial]
#[allow(clippy::unwrap_used)]
pub fn test_object() {
    crate::test::logging::init_log();

    let fs = crate::test::fsinit::create_fs().unwrap();

    let mut buf = [0u8;4];
    assert_eq!(fs.read_object(0,0,&mut buf).unwrap(),0);
}

#[test]
#[serial]
#[allow(clippy::unwrap_used)]
pub fn test_insert() {
    crate::test::logging::init_log();

    let fs = crate::test::fsinit::create_fs().unwrap();

    let a1 = fs.write().unwrap().alloc(1).unwrap().unwrap();
    let a2 = fs.write().unwrap().alloc(1).unwrap().unwrap();
    let a3 = fs.write().unwrap().alloc(1).unwrap().unwrap();
    let a4 = fs.write().unwrap().alloc(1).unwrap().unwrap();

    fs.write().unwrap().get_objects_mut().unwrap().set_object(0,Object{frags:vec![Fragment{size:1,offset:0,pointer:a1}]}).unwrap();
    fs.write().unwrap().get_objects_mut().unwrap().set_object(1,Object{frags:vec![Fragment{size:2,offset:0,pointer:a2}]}).unwrap();
    fs.write().unwrap().get_objects_mut().unwrap().set_object(2,Object{frags:vec![Fragment{size:3,offset:0,pointer:a3}]}).unwrap();
    fs.write().unwrap().get_objects_mut().unwrap().set_object(3,Object{frags:vec![Fragment{size:4,offset:0,pointer:a4}]}).unwrap();
    fs.sync().unwrap();
    assert_eq!(fs.size_object(0).unwrap(),1);
    assert_eq!(fs.size_object(1).unwrap(),2);
    assert_eq!(fs.size_object(2).unwrap(),3);
    assert_eq!(fs.size_object(3).unwrap(),4);
    assert_eq!(fs.write_object(0,0,&[0]).unwrap(),1);
    assert_eq!(fs.write_object(1,0,&[0,1]).unwrap(),2);
    assert_eq!(fs.write_object(2,0,&[0,1,2]).unwrap(),3);
    assert_eq!(fs.write_object(3,0,&[0,1,2,3]).unwrap(),4);
    let mut buf = [0u8;4];
    assert_eq!(fs.read_object(0,0,&mut buf[0..1]).unwrap(),1);
    assert_eq!(buf,[0u8,0u8,0u8,0u8]);
    assert_eq!(fs.read_object(1,0,&mut buf[0..2]).unwrap(),2);
    assert_eq!(buf,[0u8,1u8,0u8,0u8]);
    assert_eq!(fs.read_object(2,0,&mut buf[0..3]).unwrap(),3);
    assert_eq!(buf,[0u8,1u8,2u8,0u8]);
    assert_eq!(fs.read_object(3,0,&mut buf[0..4]).unwrap(),4);
    assert_eq!(buf,[0u8,1u8,2u8,3u8]);
    fs.commit().unwrap();
}