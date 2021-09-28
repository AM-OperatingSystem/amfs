#![allow(unknown_lints)]
#![allow(require_stability_comment)]
#![cfg(not(tarpaulin_include))]
use amfs::operations::mkfs_single;

fn main() {
    let d = amfs::DiskFile::open("test.img").unwrap();
    mkfs_single(d).unwrap();
}
