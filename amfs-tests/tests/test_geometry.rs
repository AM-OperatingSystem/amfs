use amfs::{DiskFile, Superblock};
use amfs_macros::*;
use amfs_tests::imagegen::generators::*;

#[test_fs]
fn test_geometry() {
    generate_image!(5);

    let d = load_image!(5);
    let sb_locs = d.get_header_locs().unwrap();
    let sbs: Vec<Superblock> = sb_locs
        .iter()
        .map(|x| Superblock::read(d.clone(), *x).unwrap())
        .collect();

    for sb in sbs {
        let geo = sb.get_geometry(d.clone(), 0).unwrap();
        let gdid = geo.device_ids[0];
        let sdid = sb.devid();
        assert_eq!(gdid, sdid);
    }
}
