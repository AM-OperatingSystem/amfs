use crate::{Disk,DiskGroup,AMPointerLocal,AMPointerGlobal,FSHandle};

use crate::SIGNATURE;

use std::collections::BTreeSet;

use std::convert::TryInto;

use bitvec::prelude::*;

#[derive(Debug)]
pub enum FSCKErrorLoc {
    Local(AMPointerLocal),
    Global(AMPointerGlobal),
}

impl std::convert::From<AMPointerGlobal> for FSCKErrorLoc {
    fn from(p: AMPointerGlobal) -> Self {
        Self::Global(p)
    }
}

impl std::convert::From<AMPointerLocal> for FSCKErrorLoc {
    fn from(p: AMPointerLocal) -> Self {
        Self::Local(p)
    }
}

#[derive(Debug)]
pub enum FSCKErrorKind {
    InvalidSuperblock,
    MismatchedSuperblock,
    InvalidGeometry,
    InvalidRoot,
}

#[derive(Debug)]
pub struct FSCKError {
    location: FSCKErrorLoc,
    kind: FSCKErrorKind,
}

macro_rules! return_error {
    ($loc:expr, $err:expr) => {
        return Err(FSCKError{location:$loc.into(),kind:$err})
    }
}

/// Checks the filesystem on a single disk
#[cfg(feature = "unstable")]
pub fn fsck_single_scan(d: Disk) -> Result<(),FSCKError> {

    let mut blockmap = bitvec![0; d.size().expect("Disk error").try_into().expect("Bitness error")];

    let fs = FSHandle::open(&[d.clone()]).ok();

    let sb_locs = d.get_header_locs().expect("Disk error");
    info!("Verifying superblocks...");
    let mut geom_locs : BTreeSet<AMPointerLocal>= BTreeSet::new();
    let mut root_locs : BTreeSet<AMPointerGlobal>= BTreeSet::new();
    let mut d_id = None;
    for loc in sb_locs {
        blockmap.set(loc.loc().try_into().unwrap(),true);
        info!("\tVerifying superblock at {}",loc);
        let sb = crate::Superblock::read(d.clone(), loc).ok();
        let sb = if let Some(sb) = sb {
                for i in 0..16 {
                    geom_locs.insert(sb.geometries(i));
                }
                for i in 0..128 {
                    root_locs.insert(sb.rootnodes(i));
                }
                d_id = Some(sb.devid());
                info!("\t\tOK!");
                sb
            } else {
                warn!("\t\tNot OK");
                let mut sb = unsafe { crate::Superblock::read_unchecked(d.clone(), loc).expect("Disk error") } ;
                if sb.signature() != SIGNATURE {
                    warn!("\t\t\tIncorrect signature");
                }
                if !sb.verify_checksum() {
                    warn!("\t\t\tIncorrect checksum");
                }
                return_error!(loc,FSCKErrorKind::InvalidSuperblock);
            };
        if let Some(fs) = &fs {
            if sb.devid() != fs.read().expect("Poisoned mutex").get_superblock().expect("Invalid superblock").devid() {
                warn!("\t\t\tMismatched device ID");
                return_error!(loc,FSCKErrorKind::MismatchedSuperblock);
            }
            if sb.features() != fs.read().expect("Poisoned mutex").get_superblock().expect("Invalid superblock").features() {
                warn!("\t\t\tMismatched feature flags");
                return_error!(loc,FSCKErrorKind::MismatchedSuperblock);
            }
            if sb.latest_root() != fs.read().expect("Poisoned mutex").get_superblock().expect("Invalid superblock").latest_root() {
                warn!("\t\t\tMismatched latest root index");
                return_error!(loc,FSCKErrorKind::MismatchedSuperblock);
            }
            /*for i in 0..16 {
                if sb.geometries(i) != fs.read().expect("Poisoned mutex").get_superblock().unwrap().geometries(i) {
                    warn!("\t\t\tMismatched geometry {}",i);
                }
            }*/
            for i in 0..128 {
                if sb.rootnodes(i) != fs.read().expect("Poisoned mutex").get_superblock().expect("Invalid superblock").rootnodes(i) {
                    warn!("\t\t\tMismatched root node {}",i);
                    return_error!(loc,FSCKErrorKind::MismatchedSuperblock);
                }
            }
        }
    }
    let mut d_geo = None;
    info!("Verifying geometries...");
    for loc in geom_locs {
        if loc.is_null() { continue; }
        blockmap.set(loc.loc().try_into().expect("Bitness error"),true);
        info!("\tVerifying geometry at {}",loc);
        let geo = crate::Geometry::read(d.clone(), loc).ok();
        if let Some(geo) = geo {
            d_geo = Some(geo);
            info!("\t\tOK!");
        } else {
            warn!("\t\tNot OK");
            return_error!(loc,FSCKErrorKind::InvalidGeometry);
        }
    }
    let dgs = DiskGroup::from_geo(d_geo.expect("No intact geometry"), &[d_id.expect("No intact superblock")], &[d]);
    info!("Verifying roots...");
    for loc in root_locs {
        if loc.is_null() { continue; }
        if loc.dev() == 0 && loc.geo() == 0 {
            blockmap.set(loc.loc().try_into().expect("Bitness error"),true);
        } else {
            warn!("We don't have a disk for {}",loc);
        }
        info!("\tVerifying rootnode at {}",loc);
        let root = crate::FSGroup::read(&[Some(dgs.clone())], loc).ok();
        if let Some(root) = root {
            info!("\t\tOK!");
        } else {
            warn!("\t\tNot OK");
            return_error!(loc,FSCKErrorKind::InvalidRoot);
        }
    }
    Ok(())
}