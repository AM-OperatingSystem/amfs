//!
//! The superblock is the highest-level data sctructure on each disk in the AMFS array.
//! 
//! There are four copies of the superblock on each disk.
//! 
//! Superblocks are stored in predetermined locations: Two in the first two blocks of the disk, two in the last two blocks:
//! ```
//! # use amfs::DiskMem;
//! # use amos_std::error::AMError;
//! let disk = DiskMem::open(1000);
//! let sb_locs = disk.get_header_locs()?;
//! assert_eq!(sb_locs[0].loc(), 0);
//! assert_eq!(sb_locs[1].loc(), 1);
//! assert_eq!(sb_locs[2].loc(), 998);
//! assert_eq!(sb_locs[3].loc(), 999);
//! # Ok::<(), AMError>(())
//! ```
//! 
//! 
//! 
//! Superblocks contain a signature to help locate them in case partition information is lost:
//! ```
//! # use amfs::Superblock;
//! # const DEVID: u64 = 1;
//! let sb = Superblock::new(DEVID);
//! assert_eq!(sb.signature(),b"amosAMFS");
//! ```
//! 
//! Each superblock contains the device ID of the disk on which it resides.
//! 
//! Superblocks contain a set of [feature flags](crate::AMFeatures), allowing combatibility tests between the on-disk format and the loaded driver.
//! 
//! Since superblocks aren't referenced by pointers, they contain a built-in checksum to enable integrity checking.
//! 
//! Superblocks contain an array of up to 16 geometries. This allows the pool to be reconfigured while online. (see [doc::geometry](crate::doc::geometry) for more details)
//! 
//! Superblocks contain an array of 128 pointers to a root [FSGroup](crate::FSGroup). When loading, if the latest is not valid, we walk backwards until we encounter a valid one.