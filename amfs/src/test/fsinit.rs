use crate::mkfs::mkfs_single;
use crate::{DiskFile,AMFS};
use amos_std::AMResult;

pub fn create_fs() -> AMResult<AMFS>{
    let d = DiskFile::open("test.img").unwrap();
    mkfs_single(d.clone()).unwrap();
    drop(d);

    let d = DiskFile::open("test.img").unwrap();
    AMFS::open(&[d])
}