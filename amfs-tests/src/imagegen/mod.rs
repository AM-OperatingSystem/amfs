pub mod generators;
mod utils;

use std::sync::Once;

static mut CHECKSUMS: Vec<String> = Vec::new();
static INIT: Once = Once::new();

pub fn get_checksums() -> &'static Vec<String> {
    unsafe {
        INIT.call_once(|| {
            CHECKSUMS = load_checksums();
        });
        &CHECKSUMS
    }
}

fn load_checksums() -> Vec<String> {
    use std::fs::File;
    use std::io::BufRead;
    let file = File::open("hashes.txt").unwrap();
    let res: Result<Vec<String>, _> = std::io::BufReader::new(file).lines().collect();
    res.unwrap()
}
