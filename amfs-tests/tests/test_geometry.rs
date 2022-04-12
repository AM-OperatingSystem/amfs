use amfs::{DiskFile, Superblock};
use amfs_macros::*;
use amfs_tests::imagegen::generators::*;

#[test_fs]
fn test_geometry() {
    generate_image!(5);

    let d = load_image!(5);
    let sb_locs = d.get_header_locs().unwrap();
    let superblocks: Vec<Superblock> = sb_locs
        .iter()
        .map(|x| Superblock::read(d.clone(), *x).unwrap())
        .collect();

    for sb in superblocks {
        let geo = sb.get_geometry(d.clone(), 0).unwrap();
        let geo_did = geo.device_ids[0];
        let sb_did = sb.devid();
        assert_eq!(geo_did, sb_did);
    }
}
