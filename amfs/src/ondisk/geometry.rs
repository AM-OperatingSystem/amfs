use std::{mem,slice};
use std::ops::{Deref,DerefMut};

use crate::{AMPointerLocal,Disk};

use amos_std::AMResult;

use crate::BLOCK_SIZE;

/// Describes the way the disks are arranged into the geometry.
#[repr(u8)]
#[derive(Copy,Clone,Debug)]
pub enum GeometryFlavor {
    /// A single disk.
    Single,
    /// Multiple striped disks.
    _Striped,
}

#[repr(packed)]
/// Represents a particular arrangement of disks into a volume
#[derive(Copy,Clone,Debug)]
pub struct Geometry {
    ///The device IDs of each disk within the arrangement
    pub device_ids: [u64;256],
    _padding: [u8; BLOCK_SIZE - 2049],
    ///The arrangement of disks within the geometry
    pub flavor: GeometryFlavor,
}

impl Geometry {
    /// Creates a new empty geometry object.
    #[cfg(feature="unstable")]
    pub fn new() -> Geometry {
        Geometry {
            flavor: GeometryFlavor::Single,
            device_ids: [0;256],
            _padding: [0; BLOCK_SIZE - 2049],
        }
    }
    /// Reads a geometry from disk.
    #[cfg(feature="stable")]
    pub fn read(mut d: Disk, ptr: AMPointerLocal) -> AMResult<Geometry> {
        let mut res: Geometry = Geometry::new();
        d.read_at(ptr.loc(), &mut res)?;
        assert!(ptr.validate(d)?);
        Ok(res)
    }
    /// Writes a geometry to disk.
    #[cfg(feature="stable")]
    pub fn write(&self, mut d: Disk, mut ptr: AMPointerLocal) -> AMResult<AMPointerLocal> {
        d.write_at(ptr.loc(), self)?;
        ptr.update(d)?;
        Ok(ptr)
    }
    /// Gets the geometry object's flavor
    #[cfg(feature="stable")]
    pub fn flavor(&self) -> GeometryFlavor {
        self.flavor
    }
}

impl Deref for Geometry {
    type Target = [u8];
    #[cfg(feature="unstable")]
    fn deref(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self as *const Geometry as *const u8, mem::size_of::<Geometry>())
                as &[u8]
        }
    }
}

impl DerefMut for Geometry {
    #[cfg(feature="unstable")]
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self as *mut Geometry as *mut u8, mem::size_of::<Geometry>())
                as &mut [u8]
        }
    }
}

#[test]
fn size_test() {
    assert_eq!(mem::size_of::<Geometry>(), BLOCK_SIZE);
}