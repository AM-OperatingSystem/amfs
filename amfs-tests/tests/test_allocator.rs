use amfs::DiskFile;
use amfs_macros::*;
use amfs_tests::{imagegen::generators::*, test_dump};

#[test_fs]
fn test_missing_allocator() {
    use amfs::{test, Superblock};

    generate_image!(6);

    let d = load_image!(6);

    let dg = test::dg::load_dg_disk_single(d.clone());

    let sb_locs = d.get_header_locs().unwrap();
    let sb = Superblock::read(d, sb_locs[0]).unwrap();
    let rg = sb.get_group(&[Some(dg.clone())]).unwrap();
    rg.get_allocators(&[Some(dg)]).unwrap();
}
