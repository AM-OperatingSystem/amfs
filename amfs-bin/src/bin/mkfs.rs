use amfs::mkfs::mkfs_single;

fn main() {
    let d = amfs::DiskFile::open("test.img").unwrap();
    mkfs_single(d).unwrap();
}