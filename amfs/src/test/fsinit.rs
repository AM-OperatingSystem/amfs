use amos_std::AMResult;
use crate::{operations::mkfs_single, DiskFile, FSHandle};

pub fn create_fs() -> AMResult<FSHandle> {
    let d = DiskFile::open("test.img").unwrap();
    mkfs_single(d.clone()).unwrap();
    drop(d);

    let d = DiskFile::open("test.img").unwrap();
    FSHandle::open(&[d])
}
