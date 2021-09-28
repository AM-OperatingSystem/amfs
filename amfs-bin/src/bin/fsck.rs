#![cfg(not(tarpaulin_include))]
#![allow(clippy::all)]
#![allow(unknown_lints)]
#![allow(require_stability_comment)]

use amfs::operations::fsck_single;
use amfs::DiskFile;

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let d = DiskFile::open(&path).unwrap();
    fsck_single(d).unwrap();
}
