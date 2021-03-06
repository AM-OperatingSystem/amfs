#![feature(assert_matches)]
#![warn(missing_docs)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::manual_flatten)]
#![allow(clippy::new_without_default)]
#![allow(clippy::upper_case_acronyms)]
#![allow(clippy::comparison_chain)]
#![allow(dead_code)]
#![warn(clippy::unwrap_used)]
#![warn(clippy::print_stdout)]
#![deny(clippy::cast_possible_truncation)]

//! AMFS, AMOS Filesystem.

#[macro_use]
extern crate more_asserts;

#[macro_use]
extern crate log;

#[macro_use]
extern crate amfs_macros;

/// The filesystem's block size. All allocations are a multiple of this size.
pub const BLOCK_SIZE: usize = 4096;

/// The filesystem's signature. Appears at the start of top-level headers.
pub const SIGNATURE: &[u8; 8] = b"amosAMFS";

use std::sync::atomic::AtomicBool;

use self::fs::AMFS;
pub use self::{
    disk::{Disk, DiskFile, DiskGroup, DiskMem},
    features::AMFeatures,
    fs::FSHandle,
    ondisk::*,
};

mod disk;
mod features;
mod fs;

mod ondisk;

/// Functions useful for testing
pub mod test;

/// Implementation for several utilities: fsck,mkfs,etc...
pub mod operations;

/// Documentation-only module
pub mod doc;

/// Converts any object into a u8 slice\
/// # Safety
/// This function is only safe for types with stable ABI representations. In practice, this means only structs with repr(C)
#[cfg(feature = "stable")]
pub unsafe fn any_as_u8_slice<T: Sized>(p: &T) -> &[u8] {
    ::std::slice::from_raw_parts((p as *const T) as *const u8, ::std::mem::size_of::<T>())
}

/// Converts a u8 slice into an object
/// # Safety
/// This function is only safe for types with stable ABI representations. In practice, this means only structs with repr(C)
#[cfg(feature = "stable")]
pub unsafe fn u8_slice_as_any<T: Sized + endian_codec::DecodeLE>(p: &[u8]) -> T {
    assert!(p.len() >= ::std::mem::size_of::<T>());
    T::decode_from_le_bytes(&p[..::std::mem::size_of::<T>()])
}

static CHECKSUMS_ENABLED: AtomicBool = AtomicBool::new(true);

/// Disable checksum verification to allow dumping/recovering a broken filesystem
/// # Safety
/// It's pretty much never safe to call this.
pub unsafe fn disable_checksums() {
    CHECKSUMS_ENABLED.store(false, std::sync::atomic::Ordering::Relaxed)
}
