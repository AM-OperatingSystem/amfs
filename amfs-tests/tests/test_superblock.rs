use amfs::DiskFile;
use amfs_macros::*;
use amfs_tests::imagegen::generators::*;
use amos_std::error::{AMError, AMErrorFS};

#[test_fs]
fn test_err_signature() {
    use amfs::Superblock;

    generate_image!(0);

    let d = load_image!(0);
    let sb_locs = d.get_header_locs().unwrap();
    for i in sb_locs {
        assert_eq!(
            Superblock::read(d.clone(), i).err(),
            Some(AMError::FS(AMErrorFS::Signature))
        );
    }
}
#[test_fs]
fn test_err_checksum() {
    use amfs::Superblock;

    generate_image!(1);

    let d = load_image!(1);
    let sb_locs = d.get_header_locs().unwrap();
    for i in sb_locs {
        assert_eq!(
            Superblock::read(d.clone(), i).err(),
            Some(AMError::FS(AMErrorFS::Checksum))
        );
    }
}

#[test_fs]
fn test_err_diskids() {
    use amfs::Superblock;

    generate_image!(2);

    let d = load_image!(2);
    let sb_locs = d.get_header_locs().unwrap();
    for i in sb_locs {
        assert_eq!(
            Superblock::read(d.clone(), i).err(),
            Some(AMError::FS(AMErrorFS::DiskID))
        );
    }
}

#[test_fs]
fn test_okay() {
    use amfs::Superblock;

    generate_image!(3);

    let d = load_image!(3);
    let sb_locs = d.get_header_locs().unwrap();
    for i in sb_locs {
        Superblock::read(d.clone(), i).unwrap();
    }
}
