#![allow(unknown_lints)]
#![allow(require_stability_comment)]

pub use fsck::fsck_single_scan;
pub use mkfs::mkfs_single;

mod fsck;
mod mkfs;
