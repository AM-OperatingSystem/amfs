#![allow(unknown_lints)]
#![allow(require_stability_comment)]

pub use mkfs::mkfs_single;
pub use fsck::fsck_single_scan;

mod mkfs;
mod fsck;