use amfs::DiskFile;
use amfs_macros::*;
use amfs_tests::{imagegen::generators::*, test_dump};
use amos_std::error::AMErrorFS;

#[test_fs]
fn test_missing_rootgroup() {
    use amfs::{test, Superblock};

    generate_image!(3);

    let d = load_image!(3);

    let dg = test::dg::load_dg_disk_single(d.clone());

    let sb_locs = d.get_header_locs().unwrap();
    for i in sb_locs {
        let sb = Superblock::read(d.clone(), i).unwrap();
        assert_eq!(
            sb.get_group(&[Some(dg.clone())])
                .err()
                .unwrap()
                .downcast::<AMErrorFS>()
                .unwrap(),
            AMErrorFS::NoFSGroup
        )
    }
}
