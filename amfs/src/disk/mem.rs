use std::{cell::RefCell, convert::TryFrom, rc::Rc};

use amos_std::{error::AMError, AMResult};

use crate::{disk::DiskObj, BLOCK_SIZE};

/// A disk object stored in a file.
pub struct DiskMem {
    data: Vec<[u8; BLOCK_SIZE]>,
    size: u64,
}

impl DiskMem {
    /// Creates a disk object using a filename.
    #[cfg(feature = "stable")]
    pub fn open(size: usize) -> super::Disk {
        let mut data = Vec::new();
        for _ in 0..size {
            data.push([0; BLOCK_SIZE]);
        }
        super::Disk(Rc::new(RefCell::new(DiskMem {
            data,
            size: size as u64,
        })))
    }
}

impl DiskObj for DiskMem {
    #[cfg(feature = "stable")]
    fn read_at(&mut self, block: u64, buffer: &mut [u8]) -> AMResult<usize> {
        buffer.copy_from_slice(
            self.data
                .get(usize::try_from(block).or(Err(AMError::TODO(0)))?)
                .ok_or(AMError::TODO(0))?,
        );
        Ok(BLOCK_SIZE)
    }
    #[cfg(feature = "stable")]
    fn write_at(&mut self, block: u64, buffer: &[u8]) -> AMResult<usize> {
        self.data[usize::try_from(block).or(Err(AMError::TODO(0)))?].copy_from_slice(buffer);
        Ok(BLOCK_SIZE)
    }
    #[cfg(feature = "stable")]
    fn size(&self) -> AMResult<u64> {
        Ok(self.size)
    }
    #[cfg(feature = "stable")]
    fn sync(&mut self) -> AMResult<()> {
        Ok(())
    }
}
