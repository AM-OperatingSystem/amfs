use std::collections::BTreeMap;
use crate::{Geometry,GeometryFlavor,Disk,AMPointerGlobal,Allocator};
use amos_std::AMResult;

/// Represents a group of disks associated with a geometry
#[derive(Derivative,Clone)]
#[derivative(Debug)]
pub struct DiskGroup {
    pub(crate) geo: Geometry,
    #[derivative(Debug="ignore")]
    disks: Vec<Disk>,
    pub(crate) allocs: Vec<Allocator>,
}

impl DiskGroup {
    /// Creates an empty disk group
    pub fn new() -> DiskGroup {
        DiskGroup{
            geo: Geometry::new(),
            disks: Vec::new(),
            allocs: Vec::new(),
        }
    }
    /// Creates a disk group containing a single disk
    pub fn single(g: Geometry, d: Disk, a: Allocator) -> DiskGroup {
        DiskGroup{
            geo: g,
            disks: vec![d],
            allocs: vec![a],
        }
    }
    /// Creates a disk group containing a single disk
    pub fn from_geo(g: Geometry, devids: &[u64], ds: &[Disk]) -> DiskGroup {
        let mut disks = Vec::new();
        for devid in g.device_ids {
            if devid == 0 { break; }
            let diskno = devids.iter().position(|r| *r == devid).expect("Superblock with devid matching no disk");
            disks.push(ds[diskno].clone());
        }
        DiskGroup{
            geo: g,
            disks,
            allocs: Vec::new(),
        }
    }
    /// Initializes out allocator set from an allocator map
    pub fn load_allocators(&mut self, allocs: BTreeMap<u64,Allocator>) {
        for devid in self.geo.device_ids {
            if devid == 0 { break; }
            self.allocs.push(allocs.get(&devid).unwrap().clone());
        }
    }
    /// Gets the nth disk
    pub fn get_disk(&self, n: u8) -> Disk{
        assert!(self.geo.device_ids[n as usize]!=0);
        self.disks[n as usize].clone()
    }
    /// Allocates a block
    pub fn alloc(&mut self, n: u64) -> AMResult<AMPointerGlobal> {
        Ok(
            match self.geo.flavor() {
                GeometryFlavor::Single => {
                    let ptr = self.allocs[0].alloc(n).ok_or(0)?;
                    AMPointerGlobal::new(ptr,1,0,0)
                },
                _ => unimplemented!(),
            }
        )
    }
    /// Allocates a block
    pub fn alloc_many(&mut self, count: u64) -> AMResult<Vec<AMPointerGlobal>> {
        Ok(
            match self.geo.flavor() {
                GeometryFlavor::Single => {
                    self.allocs[0].alloc_many(count).ok_or(0)?.iter().map(|x| AMPointerGlobal::new(*x,1,0,0)).collect()
                },
                _ => unimplemented!(),
            }
        )
    }
}

impl Default for DiskGroup {
    fn default() -> Self {
        Self::new()
    }
}