use crc32fast::Hasher;
use crate::{Disk,DiskGroup,GeometryFlavor};
use amos_std::AMResult;
use crate::BLOCK_SIZE;

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(C)]
/// AMFS local pointer. Valid within one disk.
pub struct AMPointerLocal(pub(crate) AMPointer);

#[derive(Debug, Copy, Clone, PartialEq)]
#[repr(C)]
/// AMFS global pointer. Valid within a volume.
pub struct AMPointerGlobal(pub(crate) AMPointer);

impl AMPointerGlobal {
    /// Creates a new pointer pointing at a given address and device. Invalid until updated
    pub fn new(addr: u64, len:u8, geo:u8, dev:u8) -> Self {
        Self{0:AMPointer::new(addr,len,geo,dev)}
    }
    /// Creates a null pointer. Guaranteed invalid.
    pub fn null () -> AMPointerGlobal {
        AMPointerGlobal{0:AMPointer::null()}
    }
    /// Validates a pointer against a block on-disk.
    pub fn validate(&self, d: &[Option<DiskGroup>]) -> AMResult<bool> {
        assert_eq!(self.0.len,1);
        let mut buf = [0;BLOCK_SIZE];
        self.read(0,BLOCK_SIZE,d, &mut buf)?;
        Ok(self.0.validate(&buf))
    }
    /// Updates a pointer's checksum to match on-disk data.
    pub fn update(&mut self, d: &[Option<DiskGroup>]) -> AMResult<()> {
        assert_eq!(self.0.len,1);
        let mut buf = [0;BLOCK_SIZE];
        self.read(0,BLOCK_SIZE,d, &mut buf)?;
        self.0.update(&buf);
        Ok(())
    }
    /// Gets the pointer's geometry
    pub fn geometry(&self) -> u8 {
        self.0.geometry
    }
    /// Checks if the pointer is null
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
    /// Gets the location the pointer is addressing
    pub fn loc(&self) -> u64 {
        assert!(!self.is_null());
        self.0.location
    }
    /// Reads from the referenced location
    pub fn read(self, start: usize, size: usize, dgs: &[Option<DiskGroup>], buf: &mut [u8]) -> AMResult<usize> {
        assert_eq!(self.0.len,1);
        assert_eq!(start,0);
        assert_eq!(size,BLOCK_SIZE);
        match dgs[self.geometry() as usize].as_ref().ok_or(0)?.geo.flavor() {
            GeometryFlavor::Single => {
                dgs[self.geometry() as usize].as_ref().ok_or(0)?.get_disk(0).read_at(self.loc(),buf)
            },
            _ => unimplemented!(),
        }
    }
    /// Reads from the referenced location
    pub fn read_vec(self, dgs: &[Option<DiskGroup>]) -> AMResult<Vec<u8>> {
        let mut res = Vec::new();
        res.resize(usize::from(self.0.len)*BLOCK_SIZE,0);
        self.read(0,usize::from(self.0.len)*BLOCK_SIZE,dgs,res.as_mut_slice())?;
        Ok(res)
    }
    /// Writes to the referenced location
    pub fn write(self, start: usize, size: usize, dgs: &[Option<DiskGroup>], buf: &[u8]) -> AMResult<usize> {
        assert_eq!(self.0.len,1);
        assert_eq!(start,0);
        assert_eq!(size,BLOCK_SIZE);
        match dgs[self.geometry() as usize].as_ref().ok_or(0)?.geo.flavor() {
            GeometryFlavor::Single => {
                dgs[self.geometry() as usize].as_ref().ok_or(0)?.get_disk(0).write_at(self.loc(),buf)
            },
            _ => unimplemented!(),
        }
    }
    /// Creates a pointer from an array of bytes
    pub fn from_bytes(buf: [u8;16]) -> AMPointerGlobal {
        AMPointerGlobal{0:AMPointer::from_bytes(buf)}
    }
}

impl AMPointerLocal {
    /// Creates a new pointer pointing at a given address. Invalid until updated
    pub fn new(addr: u64) -> AMPointerLocal {
        AMPointerLocal{0:AMPointer::new(addr,0,1,0)}
    }
    /// Creates a null pointer. Guaranteed invalid.
    pub fn null() -> AMPointerLocal {
        AMPointerLocal{0:AMPointer::null()}
    }
    /// Checks if the pointer is null
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
    /// Validates a pointer against a block on-disk.
    pub fn validate(&self, mut d: Disk) -> AMResult<bool> {
        let mut target = [0;BLOCK_SIZE];
        d.read_at(self.0.location,&mut target)?;
        Ok(self.0.validate(&target))
    }
    /// Updates a pointer's checksum to match on-disk data.
    pub fn update(&mut self, mut d: Disk) -> AMResult<()> {
        let mut target = [0;BLOCK_SIZE];
        d.read_at(self.0.location,&mut target)?;
        self.0.update(&target);
        Ok(())
    }
    /// Gets the location the pointer is addressing
    pub fn loc(&self) -> u64 {
        assert!(!self.is_null());
        self.0.location
    }
    /// Sets the location the pointer is addressing
    pub fn set_loc(&mut self, loc: u64) {
        self.0.padding = 0xFF;
        self.0.location = loc;
    }
    /// Creates a pointer from an array of bytes
    pub fn from_bytes(buf: [u8;16]) -> AMPointerLocal {
        AMPointerLocal{0:AMPointer::from_bytes(buf)}
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
pub(crate) struct AMPointer {
    location: u64,
    checksum: u32,
    device: u8,
    geometry: u8,
    len: u8,
    padding: u8,
}

impl AMPointer {
    pub fn new (addr: u64, len: u8, geo: u8, dev: u8) -> AMPointer {
        AMPointer{
            location: addr,
            device: dev,
            geometry: geo,
            len,
            padding: 0xFF,
            checksum: 0,
        }
    }
    pub fn null() -> AMPointer {
        AMPointer{
            location: 0,
            device: 0,
            geometry: 0x7F,
            len: 0,
            padding: 0,
            checksum: 0,
        }
    }
    pub fn is_null(&self) -> bool {
        self.padding==0
    }
    pub fn validate(&self, target: &[u8]) -> bool {
        let mut hasher = Hasher::new();
        hasher.update(target);
        let checksum = hasher.finalize();
        if checksum != self.checksum {
            return false;
        }
        true
    }

    pub fn update(&mut self, target: &[u8]) {
        let mut hasher = Hasher::new();
        hasher.update(target);
        let checksum = hasher.finalize();
        self.checksum = checksum;
    }

    pub fn from_bytes(buf: [u8;16]) -> AMPointer {
        unsafe { std::ptr::read(buf.as_ptr() as *const _) }
    }
}

#[cfg(test)]
use std::mem;

#[test]
fn size_test() {
    assert_eq!(mem::size_of::<AMPointer>(), 16);
}

#[test]
fn test_checksum() {
    let mut p = AMPointer::null();
    let data = [0;4096];
    assert!(!p.validate(&data));
    p.update(&data);
    assert!(p.validate(&data));
}