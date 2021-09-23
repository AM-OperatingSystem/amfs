use crate::{AMPointerGlobal};

/// A journal entry stores the information necessary to recreate a fs operation.
#[derive(Debug)]
pub enum JournalEntry {
    /// A block has been allocated
    Alloc(AMPointerGlobal),
    /// A block has been freed
    Free(AMPointerGlobal),
}