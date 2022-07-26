use amfs::DiskFile;
use amfs_macros::*;
use amfs_tests::{imagegen::generators::*, test_dump};
use amos_std::error::AMErrorFS;

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

#[test_fs]
fn test_empty_allocatorlist() {
    use amfs::{test, Superblock};

    generate_image!(7);

    let d = load_image!(7);

    let dg = test::dg::load_dg_disk_single(d.clone());

    let sb_locs = d.get_header_locs().unwrap();
    let sb = Superblock::read(d, sb_locs[0]).unwrap();
    let rg = sb.get_group(&[Some(dg.clone())]).unwrap();
    rg.get_allocators(&[Some(dg)]).unwrap();
}

#[test_fs]
fn test_empty_allocator() {
    use amfs::{test, Superblock};

    generate_image!(8);

    let d = load_image!(8);

    let dg = test::dg::load_dg_disk_single(d.clone());

    let sb_locs = d.get_header_locs().unwrap();
    let sb = Superblock::read(d, sb_locs[0]).unwrap();
    let rg = sb.get_group(&[Some(dg.clone())]).unwrap();

    assert_eq!(
        rg.get_allocators(&[Some(dg)])
            .err()
            .unwrap()
            .downcast::<AMErrorFS>()
            .unwrap(),
        AMErrorFS::NoAllocator
    );
}

#[test_fs]
fn test_allocator() {
    use amfs::{test, Superblock};

    generate_image!(9);

    let d = load_image!(9);

    let dg = test::dg::load_dg_disk_single(d.clone());

    let sb_locs = d.get_header_locs().unwrap();
    let sb = Superblock::read(d, sb_locs[0]).unwrap();
    let rg = sb.get_group(&[Some(dg.clone())]).unwrap();

    rg.get_allocators(&[Some(dg)]).unwrap();
}
