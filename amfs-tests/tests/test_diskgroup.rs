use amfs::{DiskFile, DiskGroup, Geometry, Superblock};
use amfs_macros::*;
use amfs_tests::imagegen::generators::*;
use amos_std::error::AMErrorFS;

#[test_fs]
fn test_diskgroup_mismatch() {
    generate_image!(4);

    let d = load_image!(4);
    let sb_locs = d.get_header_locs().unwrap();
    let superblocks: Vec<Superblock> = sb_locs
        .iter()
        .map(|x| Superblock::read(d.clone(), *x).unwrap())
        .collect();

    let geometries: Vec<Geometry> = superblocks
        .iter()
        .map(|x| x.get_geometry(d.clone(), 0).unwrap())
        .collect();

    assert_eq!(
        DiskGroup::from_geo(geometries[0], &[superblocks[0].devid()], &[d])
            .err()
            .unwrap()
            .downcast::<AMErrorFS>()
            .unwrap(),
        AMErrorFS::UnknownDevId
    );
}

#[test_fs]
fn test_diskgroup() {
    generate_image!(5);

    let d = load_image!(5);
    let sb_locs = d.get_header_locs().unwrap();
    let superblocks: Vec<Superblock> = sb_locs
        .iter()
        .map(|x| Superblock::read(d.clone(), *x).unwrap())
        .collect();

    let geometries: Vec<Geometry> = superblocks
        .iter()
        .map(|x| x.get_geometry(d.clone(), 0).unwrap())
        .collect();

    let _dg = DiskGroup::from_geo(geometries[0], &[superblocks[0].devid()], &[d]).unwrap();
}
