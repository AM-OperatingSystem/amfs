use amfs::{DiskFile, Superblock};
use amfs_macros::*;
use amfs_tests::{imagegen::generators::*, test_dump};
use amos_std::error::AMErrorFS;

#[test_fs]
fn test_err_signature() {
    generate_image!(0);

    let d = load_image!(0);
    let sb_locs = d.get_header_locs().unwrap();
    for i in sb_locs {
        assert_eq!(
            Superblock::read(d.clone(), i)
                .err()
                .unwrap()
                .downcast::<AMErrorFS>()
                .unwrap(),
            AMErrorFS::Signature
        );
    }
}

#[test_fs]
fn test_err_checksum() {
    generate_image!(1);

    let d = load_image!(1);
    let sb_locs = d.get_header_locs().unwrap();
    for i in sb_locs {
        assert_eq!(
            Superblock::read(d.clone(), i)
                .err()
                .unwrap()
                .downcast::<AMErrorFS>()
                .unwrap(),
            AMErrorFS::Checksum
        );
    }
}

#[test_fs]
fn test_err_diskids() {
    generate_image!(2);

    let d = load_image!(2);
    let sb_locs = d.get_header_locs().unwrap();
    for i in sb_locs {
        assert_eq!(
            Superblock::read(d.clone(), i)
                .err()
                .unwrap()
                .downcast::<AMErrorFS>()
                .unwrap(),
            AMErrorFS::DiskID
        );
    }
}

#[test_fs]
fn test_okay() {
    generate_image!(3);

    let d = load_image!(3);
    let sb_locs = d.get_header_locs().unwrap();
    for i in sb_locs {
        Superblock::read(d.clone(), i).unwrap();
    }
}
