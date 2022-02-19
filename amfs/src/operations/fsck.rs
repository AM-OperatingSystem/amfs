#![cfg(not(tarpaulin_include))]

use std::{collections::BTreeSet, convert::TryInto};

use bitvec::prelude::*;

use crate::{
    AMPointerGlobal, AMPointerLocal, AllocListEntry, Allocator, Disk, DiskGroup, FSHandle,
    FreeQueueEntry, LinkedListGlobal, SIGNATURE,
};

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
    InvalidObjectSet,
}

#[derive(Debug)]
pub struct FSCKError {
    location: FSCKErrorLoc,
    kind:     FSCKErrorKind,
}

macro_rules! return_error {
    ($loc:expr, $err:expr) => {
        if cfg!(feature = "halt_on_err") {
            return Err(FSCKError {
                location: $loc.into(),
                kind:     $err,
            });
        }
    };
}

macro_rules! return_error_always {
    ($loc:expr, $err:expr) => {
        return Err(FSCKError {
            location: $loc.into(),
            kind:     $err,
        });
    };
}

/// Checks the filesystem on a single disk
#[cfg(feature = "unstable")]
pub fn fsck_single_scan(d: Disk) -> Result<(), FSCKError> {
    let mut allocs_ok = true;

    let mut blockmap = BitVec::<u8, Msb0>::new();
    blockmap.resize(
        d.size()
            .expect("Disk error")
            .try_into()
            .expect("Bitness error"),
        false,
    );

    let fs = FSHandle::open(&[d.clone()]).ok();

    let sb_locs = d.get_header_locs().expect("Disk error");
    info!("Verifying superblocks...");
    let mut geom_locs = BTreeSet::new();
    let mut root_locs = BTreeSet::new();
    let mut d_id = None;
    for loc in sb_locs {
        blockmap.set(loc.loc().try_into().expect("E"), true);
        info!("\tVerifying superblock at {}", loc);
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
            //allocs_ok=false;
            let mut sb = unsafe { crate::Superblock::read_unchecked(d, loc).expect("Disk error") };
            if sb.signature() != SIGNATURE {
                warn!("\t\t\tIncorrect signature");
            }
            if !sb.verify_checksum() {
                warn!("\t\t\tIncorrect checksum");
            }
            return_error_always!(loc, FSCKErrorKind::InvalidSuperblock);
        };
        if let Some(fs) = &fs {
            if sb.devid()
                != fs
                    .read()
                    .expect("Poisoned mutex")
                    .get_superblock()
                    .expect("Invalid superblock")
                    .devid()
            {
                warn!("\t\t\tMismatched device ID");
                allocs_ok = false;
                return_error!(loc, FSCKErrorKind::MismatchedSuperblock);
            }
            if sb.features()
                != fs
                    .read()
                    .expect("Poisoned mutex")
                    .get_superblock()
                    .expect("Invalid superblock")
                    .features()
            {
                warn!("\t\t\tMismatched feature flags");
                allocs_ok = false;
                return_error!(loc, FSCKErrorKind::MismatchedSuperblock);
            }
            if sb.latest_root()
                != fs
                    .read()
                    .expect("Poisoned mutex")
                    .get_superblock()
                    .expect("Invalid superblock")
                    .latest_root()
            {
                warn!("\t\t\tMismatched latest root index");
                allocs_ok = false;
                return_error!(loc, FSCKErrorKind::MismatchedSuperblock);
            }
            for i in 0..128 {
                if sb.rootnodes(i)
                    != fs
                        .read()
                        .expect("Poisoned mutex")
                        .get_superblock()
                        .expect("Invalid superblock")
                        .rootnodes(i)
                {
                    warn!("\t\t\tMismatched root node {}", i);
                    allocs_ok = false;
                    return_error!(loc, FSCKErrorKind::MismatchedSuperblock);
                }
            }
        }
    }
    let mut d_geo = None;
    info!("Verifying geometries...");
    for loc in geom_locs {
        if loc.is_null() {
            continue;
        }
        blockmap.set(loc.loc().try_into().expect("Bitness error"), true);
        info!("\tVerifying geometry at {}", loc);
        let geo = crate::Geometry::read(d.clone(), loc).ok();
        if let Some(geo) = geo {
            d_geo = Some(geo);
            info!("\t\tOK!");
        } else {
            warn!("\t\tNot OK");
            return_error!(loc, FSCKErrorKind::InvalidGeometry);
        }
    }
    let dgs = DiskGroup::from_geo(
        d_geo.expect("No intact geometry"),
        &[d_id.expect("No intact superblock")],
        &[d.clone()],
    )
    .expect("Could not load diskgroup");
    info!("Verifying roots...");
    let mut alloclist_locs = BTreeSet::new();
    let mut objectset_locs = BTreeSet::new();
    let mut freequeue_locs = BTreeSet::new();
    for loc in root_locs {
        if loc.is_null() {
            continue;
        }
        if loc.dev() == 0 && loc.geo() == 0 {
            blockmap.set(loc.loc().try_into().expect("Bitness error"), true);
        } else {
            warn!("We don't have a disk for {}", loc);
        }
        info!("\tVerifying rootnode at {}", loc);
        let root = crate::FSGroup::read(&[Some(dgs.clone())], loc).ok();
        if let Some(root) = root {
            info!("\t\tOK!");
            alloclist_locs.insert(root.alloc());
            objectset_locs.insert(root.objects());
            if !root.free_queue().is_null() {
                freequeue_locs.insert(root.free_queue());
            }
        } else {
            warn!("\t\tNot OK");
            return_error!(loc, FSCKErrorKind::InvalidRoot);
            allocs_ok = false;
        }
    }
    info!("Verifying objectsets...");
    let mut objects = BTreeSet::new();
    for loc in objectset_locs {
        if loc.is_null() {
            continue;
        }
        if loc.dev() == 0 && loc.geo() == 0 {
            blockmap.set(loc.loc().try_into().expect("Bitness error"), true);
        } else {
            warn!("We don't have a disk for {}", loc);
        }
        info!("\tVerifying objectset at {}", loc);
        let objs = crate::ObjectSet::read(
            vec![
                Some(dgs.clone()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ],
            loc,
        )
        .get_objects()
        .ok();
        if let Some(objs) = objs {
            info!("\t\tOK!");
            for (i, o) in objs {
                objects.insert((i, o));
            }
        } else {
            warn!("\t\tNot OK");
            return_error!(loc, FSCKErrorKind::InvalidObjectSet);
        }
    }
    info!("Verifying objects...");
    for (id, obj) in objects {
        for frag in obj.frags() {
            info!("\tVerifying object {}, fragment at {}", id, frag.pointer);
            if frag.pointer.validate(&[Some(dgs.clone())]).expect("E") {
                info!("\t\tOK!");
            } else {
                warn!("\t\tNot OK!");
            }
            blockmap.set(frag.pointer.loc().try_into().expect("Bitness error"), true);
        }
    }
    info!("Verifying alloclists...");
    let mut alloc_locs = BTreeSet::new();
    for loc in alloclist_locs {
        info!("\tVerifying alloclist at {}", loc);
        let allocs: Option<Vec<AllocListEntry>> = <Vec<AllocListEntry> as LinkedListGlobal<
            Vec<AllocListEntry>,
        >>::read(&[Some(dgs.clone())], loc)
        .ok();
        if let Some(allocs) = allocs {
            info!("\t\tOK!");
            for alloc in allocs {
                alloc_locs.insert(alloc.allocator);
            }
        } else {
            warn!("\t\tNot OK!");
            allocs_ok = false;
        }
        blockmap.set(loc.loc().try_into().expect("Bitness error"), true);
    }
    info!("Verifying freequeue...");
    for loc in freequeue_locs {
        info!("\tVerifying freequeue at {}", loc);
        let queue: Option<Vec<FreeQueueEntry>> = <Vec<FreeQueueEntry> as LinkedListGlobal<
            Vec<FreeQueueEntry>,
        >>::read(&[Some(dgs.clone())], loc)
        .ok();
        if let Some(queue) = queue {
            for e in queue {
                blockmap.set(e.block.loc().try_into().expect("Bitness error"), true);
            }
            info!("\t\tOK!");
        } else {
            warn!("\t\tNot OK!");
            allocs_ok = false;
        }
        blockmap.set(loc.loc().try_into().expect("Bitness error"), true);
    }
    info!("Verifying allocators...");
    let mut allocs = Vec::new();
    for loc in alloc_locs {
        info!("\tVerifying allocator at {}", loc);
        let alloc = Allocator::read(&[Some(dgs.clone())], loc).ok();
        if let Some(alloc) = alloc {
            info!("\t\tOK!");
            allocs.push(alloc);
        } else {
            warn!("\t\tNot OK!");
            allocs_ok = false;
        }
        blockmap.set(loc.loc().try_into().expect("Bitness error"), true);
    }
    if allocs_ok {
        info!("Reconciling claimed blocks...");
        let mut blockmap_alloc = BitVec::<u8, Msb0>::new();
        blockmap_alloc.resize(
            d.size()
                .expect("Disk error")
                .try_into()
                .expect("Bitness error"),
            false,
        );
        for alloc in allocs {
            for (idx, ext) in alloc.extents() {
                if ext.used {
                    for i in 0..ext.size {
                        blockmap_alloc.set((idx + i).try_into().expect("Bitness error"), true);
                    }
                }
            }
        }
        let mut ok = true;
        for i in 0..blockmap.len() {
            if blockmap[i] && !blockmap_alloc[i] {
                error!("\tBlock {} in use but unclaimed", i);
                ok = false;
            }
            if !blockmap[i] && blockmap_alloc[i] {
                warn!("\tBlock {} unused but claimed", i);
                ok = false;
            }
        }
        if ok {
            info!("\tOK!");
        }
    }

    Ok(())
}
