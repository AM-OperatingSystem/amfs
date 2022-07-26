use endian_codec::{DecodeLE, PackedSize};

use crate::AMPointerGlobal;

/// A journal entry stores the information necessary to recreate a fs operation.
#[derive(Debug)]
pub enum JournalEntry {
    /// The filesystem has been mounted
    Mount,
    /// A block has been allocated
    Alloc(AMPointerGlobal),
    /// A block has been freed
    Free(AMPointerGlobal),
}

#[repr(C)]
#[derive(PackedSize, DecodeLE)]
pub struct JournalHeader {
    prev:     AMPointerGlobal,
    count:    u64,
    checksum: u32,
    _padding: u32,
}

#[test]
fn size_test() {
    use std::mem;
    assert_eq!(mem::size_of::<JournalHeader>(), 32);
}
