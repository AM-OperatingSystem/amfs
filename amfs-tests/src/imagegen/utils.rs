use std::fs::File;
use std::convert::TryInto;

use amfs::BLOCK_SIZE;

pub fn create_file (f: &File, n:usize) {
    f.set_len(0).unwrap();
    f.set_len((n*BLOCK_SIZE).try_into().unwrap()).unwrap();
}

pub fn get_disk (f: &File) -> amfs::Disk {
    amfs::DiskFile::open_file(f.try_clone().unwrap()).unwrap()
}