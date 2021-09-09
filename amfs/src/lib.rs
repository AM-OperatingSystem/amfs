#![warn(missing_docs)]
#![feature(assert_matches)]

#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![warn(clippy::unwrap_used)]
#![deny(clippy::cast_possible_truncation)]

//! AMFS, AMOS Filesystem.

#[allow(unused_imports)]
#[macro_use]
extern crate ntest;

#[macro_use]
extern crate more_asserts;

#[allow(unused_imports)]
#[macro_use]
extern crate serial_test;

#[macro_use]
extern crate log;

#[macro_use]
extern crate derivative;

#[macro_use]
extern crate amfs_macros;

/// The filesystem's block size. All allocations are a multiple of this size.
pub const BLOCK_SIZE: usize = 4096;

/// The filesystem's signature. Appears at the start of top-level headers.
pub const SIGNATURE: &[u8; 8] = b"amosAMFS";

pub use self::fs::AMFS;
pub use self::disk::{Disk,DiskFile,DiskMem,DiskGroup};
pub use self::features::AMFeatures;

pub use self::ondisk::*;

mod fs;
mod disk;
mod features;

mod ondisk;

mod test;

/// Creates a filesystem on one or more disks.
pub mod mkfs;

/// Documentation-only module
pub mod doc;