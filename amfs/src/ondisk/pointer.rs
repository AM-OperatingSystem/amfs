use std::{
    convert::{TryFrom, TryInto},
    fmt,
};

use amos_std::{error::AMError, AMResult};
use crc32fast::Hasher;
use endian_codec::{DecodeLE, PackedSize};

use crate::{Disk, DiskGroup, GeometryFlavor, BLOCK_SIZE};

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
/// AMFS local pointer. Valid within one disk.
pub struct AMPointerLocal(pub(crate) AMPointer);

impl PackedSize for AMPointerLocal {
    const PACKED_LEN: usize = AMPointer::PACKED_LEN;
}

impl DecodeLE for AMPointerLocal {
    fn decode_from_le_bytes(bytes: &[u8]) -> Self {
        AMPointerLocal(AMPointer::decode_from_le_bytes(bytes))
    }
}

impl fmt::Display for AMPointerLocal {
    #[cfg(feature = "unstable")]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            write!(f, "Local(NULL)")
        } else {
            write!(f, "Local({})", self.loc())
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
/// AMFS global pointer. Valid within a volume.
pub struct AMPointerGlobal(pub(crate) AMPointer);

impl PackedSize for AMPointerGlobal {
    const PACKED_LEN: usize = AMPointer::PACKED_LEN;
}

impl DecodeLE for AMPointerGlobal {
    fn decode_from_le_bytes(bytes: &[u8]) -> Self {
        AMPointerGlobal(AMPointer::decode_from_le_bytes(bytes))
    }
}

impl fmt::Display for AMPointerGlobal {
    #[cfg(feature = "unstable")]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            write!(f, "Global(NULL)")
        } else {
            write!(f, "Global({},{})", self.dev(), self.loc())
        }
    }
}

impl AMPointerGlobal {
    /// Creates a new pointer pointing at a given address and device. Invalid until updated
    #[cfg(feature = "stable")]
    pub fn new(addr: u64, len: u8, geo: u8, dev: u8) -> Self {
        Self(AMPointer::new(addr, len, geo, dev))
    }
    /// Creates a null pointer. Guaranteed invalid.
    #[cfg(feature = "stable")]
    pub fn null() -> AMPointerGlobal {
        AMPointerGlobal(AMPointer::null())
    }
    /// Validates a pointer against a block on-disk.
    #[cfg(feature = "unstable")]
    pub fn validate(&self, d: &[Option<DiskGroup>]) -> AMResult<bool> {
        if self.is_null() {
            return Ok(false);
        }
        assert_eq!(self.0.len, 1);
        let mut buf = [0; BLOCK_SIZE];
        self.read(0, BLOCK_SIZE, d, &mut buf)?;
        Ok(self.0.validate(&buf))
    }
    /// Updates a pointer's checksum to match on-disk data.
    #[cfg(feature = "unstable")]
    pub fn update(&mut self, d: &[Option<DiskGroup>]) -> AMResult<()> {
        assert_eq!(self.0.len, 1);
        let mut buf = [0; BLOCK_SIZE];
        self.read(0, BLOCK_SIZE, d, &mut buf)?;
        self.0.update(&buf);
        Ok(())
    }
    /// Checks if the pointer is null
    #[cfg(feature = "stable")]
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
    /// Gets the location the pointer is addressing
    #[cfg(feature = "stable")]
    pub fn loc(&self) -> u64 {
        assert!(!self.is_null());
        self.0.location
    }
    /// Gets the device the pointer is addressing
    #[cfg(feature = "stable")]
    pub fn dev(&self) -> u8 {
        assert!(!self.is_null());
        self.0.device
    }
    /// Gets the geometry the pointer is addressing
    #[cfg(feature = "stable")]
    pub fn geo(&self) -> u8 {
        assert!(!self.is_null());
        self.0.geometry
    }
    /// Gets the length of the pointer
    #[cfg(feature = "stable")]
    pub fn length(&self) -> u8 {
        assert!(!self.is_null());
        self.0.len
    }
    /// Reads from the referenced location
    #[cfg(feature = "unstable")]
    pub fn read(
        self,
        start: usize,
        size: usize,
        diskgroups: &[Option<DiskGroup>],
        data: &mut [u8],
    ) -> AMResult<usize> {
        //Single whole block writes are atomic
        if start == 0 && size == BLOCK_SIZE {
            match diskgroups
                .get(self.geo() as usize)
                .ok_or(AMError::TODO(0))?
                .as_ref()
                .ok_or(AMError::TODO(0))?
                .geo
                .flavor()
            {
                GeometryFlavor::Single => diskgroups[self.geo() as usize]
                    .as_ref()
                    .ok_or(AMError::TODO(0))?
                    .get_disk(0)?
                    .read_at(self.loc(), data),
                _ => unimplemented!(), // TODO(#3): Add support for additional geometries
            }
        } else if start % BLOCK_SIZE == 0 && size == BLOCK_SIZE {
            match diskgroups[self.geo() as usize]
                .as_ref()
                .ok_or(AMError::TODO(0))?
                .geo
                .flavor()
            {
                GeometryFlavor::Single => diskgroups[self.geo() as usize]
                    .as_ref()
                    .ok_or(AMError::TODO(0))?
                    .get_disk(0)?
                    .read_at(
                        (usize::try_from(self.loc())? + start / BLOCK_SIZE).try_into()?,
                        data,
                    ),
                _ => unimplemented!(), // TODO(#3): Add support for additional geometries
            }
        } else {
            let mut buf = [0u8; BLOCK_SIZE];
            let start_block = start / BLOCK_SIZE;
            let start_offs = start % BLOCK_SIZE;
            let end_block = (start + size) / BLOCK_SIZE;
            let end_offs = (start + size) % BLOCK_SIZE;
            self.read(start_block * BLOCK_SIZE, BLOCK_SIZE, diskgroups, &mut buf)?;
            if start_block == end_block {
                let mut buf = [0u8; BLOCK_SIZE];
                self.read(start_block * BLOCK_SIZE, BLOCK_SIZE, diskgroups, &mut buf)?;
                data.clone_from_slice(&buf[start_offs..end_offs]);
                Ok(size)
            } else {
                todo!();
            }
        }
    }
    /// Reads from the referenced location
    #[cfg(feature = "stable")]
    pub fn read_vec(self, diskgroups: &[Option<DiskGroup>]) -> AMResult<Vec<u8>> {
        let mut res = Vec::new();
        res.resize(usize::from(self.0.len) * BLOCK_SIZE, 0);
        self.read(
            0,
            usize::from(self.0.len) * BLOCK_SIZE,
            diskgroups,
            res.as_mut_slice(),
        )?;
        Ok(res)
    }
    /// Writes to the referenced location
    #[cfg(feature = "unstable")]
    pub fn write(
        self,
        start: usize,
        size: usize,
        diskgroups: &[Option<DiskGroup>],
        data: &[u8],
    ) -> AMResult<usize> {
        //Single whole block writes are atomic
        if start == 0 && size == BLOCK_SIZE {
            match diskgroups[self.geo() as usize]
                .as_ref()
                .ok_or(AMError::TODO(0))?
                .geo
                .flavor()
            {
                GeometryFlavor::Single => diskgroups[self.geo() as usize]
                    .as_ref()
                    .ok_or(AMError::TODO(0))?
                    .get_disk(0)?
                    .write_at(self.loc(), data),
                _ => unimplemented!(), // TODO(#3): Add support for additional geometries
            }
        } else if start % BLOCK_SIZE == 0 && size == BLOCK_SIZE {
            match diskgroups[self.geo() as usize]
                .as_ref()
                .ok_or(AMError::TODO(0))?
                .geo
                .flavor()
            {
                GeometryFlavor::Single => diskgroups[self.geo() as usize]
                    .as_ref()
                    .ok_or(AMError::TODO(0))?
                    .get_disk(0)?
                    .write_at(
                        (usize::try_from(self.loc())? + start / BLOCK_SIZE).try_into()?,
                        data,
                    ),
                _ => unimplemented!(), // TODO(#3): Add support for additional geometries
            }
        } else {
            let mut buf = [0u8; BLOCK_SIZE];
            let start_block = start / BLOCK_SIZE;
            let start_offs = start % BLOCK_SIZE;
            let end_block = (start + size) / BLOCK_SIZE;
            let end_offs = (start + size) % BLOCK_SIZE;
            self.read(start_block * BLOCK_SIZE, BLOCK_SIZE, diskgroups, &mut buf)?;
            if start_block == end_block {
                buf[start_offs..end_offs].clone_from_slice(data);
                self.write(start_block * BLOCK_SIZE, BLOCK_SIZE, diskgroups, &buf)?;
                Ok(size)
            } else {
                todo!();
            }
        }
    }
    /// Creates a pointer from an array of bytes
    #[cfg(feature = "stable")]
    pub fn from_bytes(buf: [u8; 16]) -> AMPointerGlobal {
        AMPointerGlobal(AMPointer::from_bytes(buf))
    }

    /// Creates an array of bytes from a pointer
    #[cfg(feature = "stable")]
    pub fn as_bytes(&self) -> [u8; 16] {
        self.0.as_bytes()
    }
}

impl AMPointerLocal {
    /// Creates a new pointer pointing at a given address. Invalid until updated
    #[cfg(feature = "stable")]
    pub fn new(addr: u64) -> AMPointerLocal {
        AMPointerLocal(AMPointer::new(addr, 0, 1, 0))
    }
    /// Creates a null pointer. Guaranteed invalid.
    #[cfg(feature = "stable")]
    pub fn null() -> AMPointerLocal {
        AMPointerLocal(AMPointer::null())
    }
    /// Checks if the pointer is null
    #[cfg(feature = "stable")]
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
    /// Validates a pointer against a block on-disk.
    #[cfg(feature = "stable")]
    pub fn validate(&self, mut d: Disk) -> AMResult<bool> {
        let mut target = [0; BLOCK_SIZE];
        d.read_at(self.0.location, &mut target)?;
        Ok(self.0.validate(&target))
    }
    /// Updates a pointer's checksum to match on-disk data.
    #[cfg(feature = "stable")]
    pub fn update(&mut self, mut d: Disk) -> AMResult<()> {
        let mut target = [0; BLOCK_SIZE];
        d.read_at(self.0.location, &mut target)?;
        self.0.update(&target);
        Ok(())
    }
    /// Gets the location the pointer is addressing
    #[cfg(feature = "stable")]
    pub fn loc(&self) -> u64 {
        assert!(!self.is_null());
        self.0.location
    }
    /// Sets the location the pointer is addressing
    #[cfg(feature = "unstable")]
    pub fn set_loc(&mut self, loc: u64) {
        self.0.padding = 0xFF;
        self.0.location = loc;
    }
    /// Creates a pointer from an array of bytes
    #[cfg(feature = "stable")]
    pub fn from_bytes(buf: [u8; 16]) -> AMPointerLocal {
        AMPointerLocal(AMPointer::from_bytes(buf))
    }

    /// Creates an array of bytes from a pointer
    #[cfg(feature = "stable")]
    pub fn as_bytes(&self) -> [u8; 16] {
        self.0.as_bytes()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, PackedSize, DecodeLE)]
#[repr(C)]
pub(crate) struct AMPointer {
    location: u64,
    checksum: u32,
    device:   u8,
    geometry: u8,
    len:      u8,
    padding:  u8,
}

impl std::cmp::Ord for AMPointer {
    #[cfg(feature = "stable")]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.location, self.device, self.geometry, self.len).cmp(&(
            other.location,
            other.device,
            other.geometry,
            other.len,
        ))
    }
}

impl std::cmp::PartialOrd for AMPointer {
    #[cfg(feature = "stable")]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some((self.location, self.device, self.geometry, self.len).cmp(&(
            other.location,
            other.device,
            other.geometry,
            other.len,
        )))
    }
}

impl std::cmp::Eq for AMPointer {}

impl AMPointer {
    #[cfg(feature = "stable")]
    pub fn new(addr: u64, len: u8, geo: u8, dev: u8) -> AMPointer {
        AMPointer {
            location: addr,
            device: dev,
            geometry: geo,
            len,
            padding: 0xFF,
            checksum: 0,
        }
    }
    #[cfg(feature = "unstable")]
    pub fn null() -> AMPointer {
        AMPointer {
            location: 0,
            device:   0,
            geometry: 0x7F,
            len:      0,
            padding:  0,
            checksum: 0,
        }
    }
    #[cfg(feature = "stable")]
    pub fn is_null(&self) -> bool {
        self.padding == 0
    }
    #[cfg(feature = "stable")]
    pub fn validate(&self, target: &[u8]) -> bool {
        if !crate::CHECKSUMS_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            return true;
        }
        let mut hasher = Hasher::new();
        hasher.update(target);
        let checksum = hasher.finalize();
        if checksum != self.checksum {
            return false;
        }
        true
    }

    #[cfg(feature = "stable")]
    pub fn update(&mut self, target: &[u8]) {
        let mut hasher = Hasher::new();
        hasher.update(target);
        let checksum = hasher.finalize();
        self.checksum = checksum;
    }

    #[cfg(feature = "stable")]
    pub fn from_bytes(buf: [u8; 16]) -> AMPointer {
        unsafe { std::ptr::read(buf.as_ptr() as *const _) }
    }

    #[cfg(feature = "stable")]
    pub fn as_bytes(&self) -> [u8; 16] {
        unsafe { std::ptr::read(self as *const AMPointer as *const _) }
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
    let data = [0; 4096];
    assert!(!p.validate(&data));
    p.update(&data);
    assert!(p.validate(&data));
}
