use crate::{Allocator, Disk, DiskGroup, Geometry, GeometryFlavor};

pub fn create_dg_mem_single(size: usize) -> DiskGroup {
    let d = crate::DiskMem::open(size);

    let mut geo = Geometry::new();

    geo.device_ids[0] = 1;
    geo.flavor = GeometryFlavor::Single;

    let alloc = Allocator::new(size as u64);

    DiskGroup::single(geo, d, alloc)
}

pub fn create_dg_file_single(name: &str) -> DiskGroup {
    let d = crate::DiskFile::open(name).unwrap();

    let mut geo = Geometry::new();

    geo.device_ids[0] = 1;
    geo.flavor = GeometryFlavor::Single;

    let alloc = Allocator::new(d.size().unwrap() as u64);

    DiskGroup::single(geo, d, alloc)
}

pub fn load_dg_disk_single(d: Disk) -> DiskGroup {
    let mut geo = Geometry::new();

    geo.device_ids[0] = 1;
    geo.flavor = GeometryFlavor::Single;

    let alloc = Allocator::new(d.size().unwrap() as u64);

    DiskGroup::single(geo, d, alloc)
}
