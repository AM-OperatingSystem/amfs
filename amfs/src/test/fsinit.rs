use amos_std::{error::AMError, AMResult};
use rand::{prelude::StdRng, Rng, SeedableRng};

use crate::{operations::mkfs_single, DiskFile, FSHandle};

pub struct CleanOnDrop<T> {
    contents: T,
    file:     String,
}

impl<T> CleanOnDrop<AMResult<T>> {
    pub fn unwrap(mut self) -> CleanOnDrop<T> {
        let mut contents = AMResult::Err(AMError::Uninit.into());
        let mut file = String::new();

        std::mem::swap(&mut self.contents, &mut contents);
        std::mem::swap(&mut self.file, &mut file);

        let res = CleanOnDrop {
            contents: contents.unwrap(),
            file,
        };
        std::mem::forget(self);
        res
    }
}

impl<T> std::ops::Deref for CleanOnDrop<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.contents
    }
}

impl<T> Drop for CleanOnDrop<T> {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.file);
    }
}

pub fn create_fs() -> CleanOnDrop<AMResult<FSHandle>> {
    let id: usize = StdRng::from_entropy().gen();
    let d = DiskFile::open(&format!("{}.img", id)).unwrap();
    mkfs_single(d.clone()).unwrap();
    drop(d);

    let d = DiskFile::open(&format!("{}.img", id)).unwrap();

    CleanOnDrop {
        contents: FSHandle::open(&[d]),
        file:     format!("{}.img", id),
    }
}
