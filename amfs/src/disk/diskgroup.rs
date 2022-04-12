use std::{
    collections::BTreeMap,
    convert::{TryFrom, TryInto},
};

use amos_std::{error::AMErrorFS, AMResult};

use crate::{AMPointerGlobal, Allocator, Disk, Fragment, Geometry, GeometryFlavor, BLOCK_SIZE};

/// Represents a group of disks associated with a geometry
#[derive(Debug, Clone)]
pub struct DiskGroup {
    /// The group's geometry object
    pub geo:           Geometry,
    disks:             Vec<Disk>,
    pub(crate) allocs: Vec<Allocator>,
}

impl DiskGroup {
    /// Creates a disk group containing a single disk
    #[cfg(feature = "stable")]
    pub fn single(g: Geometry, d: Disk, a: Allocator) -> DiskGroup {
        DiskGroup {
            geo:    g,
            disks:  vec![d],
            allocs: vec![a],
        }
    }
    /// Creates a disk group containing a single disk
    #[cfg(feature = "stable")]
    pub fn from_geo(g: Geometry, devids: &[u64], ds: &[Disk]) -> AMResult<DiskGroup> {
        let mut disks = Vec::new();
        for devid in g.device_ids {
            if devid == 0 {
                break;
            }
            let disk_no = devids
                .iter()
                .position(|r| *r == devid)
                .ok_or(AMErrorFS::UnknownDevId)?;
            disks.push(ds[disk_no].clone());
        }
        Ok(DiskGroup {
            geo: g,
            disks,
            allocs: Vec::new(),
        })
    }
    /// Initializes out allocator set from an allocator map
    #[cfg(feature = "stable")]
    pub fn load_allocators(&mut self, allocs: BTreeMap<u64, Allocator>) -> AMResult<()> {
        for devid in self.geo.device_ids {
            if devid == 0 {
                break;
            }
            self.allocs
                .push(allocs.get(&devid).ok_or(AMErrorFS::NoAllocator)?.clone());
        }
        Ok(())
    }
    /// Gets the nth disk
    #[cfg(feature = "stable")]
    pub fn get_disk(&self, n: u8) -> AMResult<Disk> {
        if self.geo.device_ids[n as usize] != 0 {
            Ok(self.disks[n as usize].clone())
        } else {
            Err(AMErrorFS::DiskID.into())
        }
    }
    /// Allocates a block
    #[cfg(feature = "unstable")]
    pub fn alloc_blocks(&mut self, n: u64) -> AMResult<AMPointerGlobal> {
        Ok(match self.geo.flavor() {
            GeometryFlavor::Single => {
                let ptr = self.allocs[0].alloc_blocks(n)?;
                AMPointerGlobal::new(ptr, 1, 0, 0)
            }
            _ => unimplemented!(), // TODO(#3): Add support for additional geometries
        })
    }
    /// Allocates a block
    #[cfg(feature = "unstable")]
    pub fn alloc_bytes(&mut self, n: u64) -> AMResult<Vec<Fragment>> {
        Ok(match self.geo.flavor() {
            GeometryFlavor::Single => {
                let mut res = Vec::new();
                let mut size_rem = usize::try_from(n)?;
                loop {
                    let ptr = self.allocs[0].alloc_blocks(1)?;
                    let size_frag = if size_rem > BLOCK_SIZE {
                        BLOCK_SIZE
                    } else {
                        size_rem
                    };
                    res.push(Fragment::new(
                        size_frag.try_into()?,
                        0,
                        AMPointerGlobal::new(ptr, 1, 0, 0),
                    ));
                    if size_rem <= BLOCK_SIZE {
                        break;
                    }
                    size_rem -= BLOCK_SIZE;
                }
                res
            }
            _ => unimplemented!(), // TODO(#3): Add support for additional geometries
        })
    }
    /// Allocates a block
    #[cfg(feature = "unstable")]
    pub fn alloc_many(&mut self, count: u64) -> AMResult<Vec<AMPointerGlobal>> {
        Ok(match self.geo.flavor() {
            GeometryFlavor::Single => self.allocs[0]
                .alloc_many(count)?
                .iter()
                .map(|x| AMPointerGlobal::new(*x, 1, 0, 0))
                .collect(),
            _ => unimplemented!(), // TODO(#3): Add support for additional geometries
        })
    }
    /// Syncs the disks
    #[cfg(feature = "stable")]
    pub fn sync(&mut self) -> AMResult<()> {
        for d in &mut self.disks {
            d.sync()?;
        }
        Ok(())
    }
}
