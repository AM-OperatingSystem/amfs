use std::{mem,slice};
use std::ops::{Deref,DerefMut};

use crate::SIGNATURE;

use crate::BLOCK_SIZE;
use crate::AMPointerLocal;

use amos_std::AMResult;
use amos_std::error::AMErrorFS;

use std::collections::BTreeSet;

use bitvec::prelude::*;

use crc32fast::Hasher;

use crate::{AMPointerGlobal,AMFeatures,Geometry,Disk,DiskGroup,FSGroup};

#[repr(C)]
#[derive(Derivative,Copy,Clone)]
#[derivative(Debug)]
/// A volume superblock. Contains volume-wide information
pub struct Superblock {
    signature: [u8; 8],
    devid: u64,
    features: BitArr!(for 2048),
    pub(crate) geometries: [AMPointerLocal;16],
    checksum: u32,
    #[derivative(Debug="ignore")]
    _padding: [u8; BLOCK_SIZE - 2581],
    pub(crate) latest_root: u8,
    pub(crate) rootnodes: [AMPointerGlobal;128],
}

impl Superblock {
    /// Creates a new superblock. All pointers are initialized null.
    pub fn new(devid: u64) -> Superblock {
        Superblock {
            signature: *SIGNATURE,
            devid,
            features: AMFeatures::current(),
            geometries: [AMPointerLocal::null();16],
            latest_root: 0,
            checksum: 0,
            _padding: [0; BLOCK_SIZE - 2581],
            rootnodes: [AMPointerGlobal::null();128],
        }
    }
    /// Reads a superblock from disk.
    pub fn read(mut d: Disk, ptr: AMPointerLocal) -> AMResult<Superblock> {
        let mut res: Superblock = Superblock::new(0);
        d.read_at(ptr.loc(), &mut res)?;
        assert_or_err!(&res.signature == SIGNATURE,AMErrorFS::Signature);
        assert_or_err!(res.verify_checksum(),AMErrorFS::Checksum);
        assert_or_err!(res.devid!=0,AMErrorFS::DiskID);
        Ok(res)
    }
    /// Writes a superblock to disk.
    pub fn write(&mut self, mut d: Disk, ptr: AMPointerLocal) -> AMResult<AMPointerLocal> {
        self.update_checksum();
        d.write_at(ptr.loc(), self)?;
        Ok(ptr)
    }
    /// Verifies our checksum
    pub fn verify_checksum(&mut self) -> bool {
        let ondisk = self.checksum;
        self.checksum=0;
        let mut hasher = Hasher::new();
        hasher.update(self);
        let calc = hasher.finalize();
        self.checksum = ondisk;
        
        ondisk==calc
    }
    /// Updates our checksum
    pub fn update_checksum(&mut self) {
        self.checksum=0;
        let mut hasher = Hasher::new();
        hasher.update(self);
        let checksum = hasher.finalize();
        self.checksum = checksum;
    }
    /// Getter for devid
    pub fn devid(&self) -> u64 {
        self.devid
    }
    /// Getter for signature
    pub fn signature(&self) -> &[u8;8] {
        &self.signature
    }
    /// Fetches the geometry object for the nth geometry spec.
    pub fn get_geometry(&self, d: Disk,  n: u8) -> AMResult<Geometry> {
        let ptr = self.geometries[n as usize];
        Geometry::read(d,ptr)
    }
    /// Tests a set of feature flags for compatibility
    pub fn test_features(&self, features: BTreeSet<usize>) -> bool {
        for i in 0..2048 {
            if {self.features}[i] && ! features.contains(&i) {
                    return false;
            }
        }
        true
    }
    /// Gets the latest valid root group
    pub fn get_group(&self, d: &[Option<DiskGroup>]) -> AMResult<FSGroup> {
        for i in 0..128 {
            let ptr = self.rootnodes[((self.latest_root+i) % 128) as usize];
            if let Ok(v) = FSGroup::read(d,ptr) {
                trace!("Loaded root group {} (latest {})",((self.latest_root+i) % 128),self.latest_root);
                return Ok(v)
            }
        }
        Err(0.into())
    }
}

impl Deref for Superblock {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(self as *const Superblock as *const u8, mem::size_of::<Superblock>())
                as &[u8]
        }
    }
}

impl DerefMut for Superblock {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe {
            slice::from_raw_parts_mut(self as *mut Superblock as *mut u8, mem::size_of::<Superblock>())
                as &mut [u8]
        }
    }
}


#[test]
fn size_test() {
    assert_eq!(mem::size_of::<Superblock>(), BLOCK_SIZE);
}

#[test]
fn feature_test() {
    let sb = Superblock::new(0);
    let mut features = BTreeSet::new();
    assert!(!sb.test_features(features.clone()));
    features.insert(crate::features::AMFeatures::Base as usize);
    assert!(sb.test_features(features.clone()));
    features.insert(crate::features::AMFeatures::Never as usize);
    assert!(sb.test_features(features.clone()));
    features.remove(&(crate::features::AMFeatures::Base as usize));
    assert!(!sb.test_features(features));
}

#[test]
#[serial]
#[allow(clippy::unwrap_used)]
pub fn test_superblock() {
    crate::test::logging::init_log();

    let _fs = crate::test::fsinit::create_fs();
}