use std::fs::File;
use std::fs::OpenOptions;
use crate::disk::DiskObj;

use amos_std::AMResult;

use std::rc::Rc;
use std::cell::RefCell;

use std::convert::TryInto;

use crate::BLOCK_SIZE;

use std::io::{Read,Write,Seek,SeekFrom};

/// A disk object stored in a file.
pub struct DiskFile {
    f: File,
    size: u64,
}

impl DiskFile {
    /// Creates a disk object using a filename.
    #[cfg(feature="stable")]
    pub fn open(f: &str) -> AMResult<super::Disk> {
        let file = if std::path::Path::new(f).exists() {
            OpenOptions::new().read(true).write(true).open(f)?
        } else {
            let res = OpenOptions::new().read(true).write(true).create(true).open(f)?;
            res.set_len((100*BLOCK_SIZE).try_into().or(Err(0))?)?;
            res
        };
        let size = file.metadata()?.len();
        Ok(super::Disk{0:Rc::new(RefCell::new(DiskFile{f:file,size}))})
    }
    /// Creates a disk object using a file.
    #[cfg(feature="stable")]
    pub fn open_file(file: File) -> AMResult<super::Disk> {
        let size = file.metadata()?.len();
        Ok(super::Disk{0:Rc::new(RefCell::new(DiskFile{f:file,size}))})
    }
}

impl DiskObj for DiskFile {
    #[cfg(feature="stable")]
    fn read_at(&mut self, block: u64, buffer: &mut [u8]) -> AMResult<usize> {
        self.f.seek(SeekFrom::Start(block*(BLOCK_SIZE as u64))).or(Err(0))?;
        assert!(buffer.len() == BLOCK_SIZE);
        self.f.read_exact(buffer).or(Err(0))?;
        Ok(buffer.len())
    }
    #[cfg(feature="stable")]
    fn write_at(&mut self, block: u64, buffer: &[u8]) -> AMResult<usize> {
        self.f.seek(SeekFrom::Start(block*(BLOCK_SIZE as u64))).or(Err(0))?;
        assert!(buffer.len() == BLOCK_SIZE);
        self.f.write_all(buffer).or(Err(0))?;
        Ok(buffer.len())
    }
    #[cfg(feature="unstable")]
    fn size(&self) -> AMResult<u64> {
        Ok(self.size/(BLOCK_SIZE as u64))
    }
    #[cfg(feature="stable")]
    fn sync(&mut self) -> AMResult<()> {
        self.f.sync_all().or(Err(0))?;
        Ok(())
    }
}