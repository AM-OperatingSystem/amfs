use crate::operations::mkfs_single;
use crate::{DiskFile, FSHandle};
use amos_std::AMResult;

pub fn create_fs() -> AMResult<FSHandle> {
    let d = DiskFile::open("test.img").unwrap();
    mkfs_single(d.clone()).unwrap();
    drop(d);

    let d = DiskFile::open("test.img").unwrap();
    FSHandle::open(&[d])
}
