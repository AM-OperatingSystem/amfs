use std::{cell::RefCell, rc::Rc};

use amos_std::AMResult;

use crate::AMPointerLocal;

/// A handle to a disk
#[derive(Clone)]
pub struct Disk(pub Rc<RefCell<dyn DiskObj>>);

impl std::fmt::Debug for Disk {
    #[cfg(feature = "unstable")]
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Disk")
    }
}

impl Disk {
    /// Reads a given block into the buffer.
    #[cfg(feature = "stable")]
    pub fn read_at(&mut self, block: u64, buffer: &mut [u8]) -> AMResult<usize> {
        self.0.borrow_mut().read_at(block, buffer)
    }
    /// Writes a block to a given location.
    #[cfg(feature = "stable")]
    pub fn write_at(&mut self, block: u64, buffer: &[u8]) -> AMResult<usize> {
        self.0.borrow_mut().write_at(block, buffer)
    }
    /// Returns the size of the disk.
    #[cfg(feature = "stable")]
    pub fn size(&self) -> AMResult<u64> {
        self.0.borrow_mut().size()
    }
    /// Syncs the FS's content to disk.
    #[cfg(feature = "stable")]
    pub fn sync(&mut self) -> AMResult<()> {
        self.0.borrow_mut().sync()
    }

    /// Calculates the expected position of a disk's headers.
    #[cfg(feature = "unstable")]
    pub fn get_header_locs(&self) -> AMResult<[AMPointerLocal; 4]> {
        let mut res = [AMPointerLocal::null(); 4];
        res[0].set_loc(0);
        res[1].set_loc(1);
        res[2].set_loc(self.0.borrow().size()? - 2);
        res[3].set_loc(self.0.borrow().size()? - 1);
        Ok(res)
    }
}

/// A disk object. Has a size, supports reading/writing of blocks.
pub trait DiskObj {
    /// Reads a given block into the buffer.
    fn read_at(&mut self, block: u64, buffer: &mut [u8]) -> AMResult<usize>;
    /// Writes a block to a given location.
    fn write_at(&mut self, block: u64, buffer: &[u8]) -> AMResult<usize>;
    /// Returns the size of the disk.
    fn size(&self) -> AMResult<u64>;
    /// Syncs the FS's content to disk.
    fn sync(&mut self) -> AMResult<()>;
}

pub use diskgroup::DiskGroup;
pub use file::DiskFile;
pub use mem::DiskMem;

pub mod diskgroup;
pub mod file;
pub mod mem;
