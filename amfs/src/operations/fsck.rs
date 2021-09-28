use crate::{Disk,AMPointerLocal,FSHandle};

use crate::SIGNATURE;

use crate::test::logging::init_log;

use amos_std::AMResult;

/// Checks the filesystem on a single disk
#[cfg(feature = "unstable")]
pub fn fsck_single(d: Disk) -> AMResult<()> {
    init_log();

    let fs = FSHandle::open(&[d.clone()]).ok();

    let sb_locs = d.get_header_locs()?;
    info!("Verifying superblocks...");
    let mut geom_locs : Vec<Vec<AMPointerLocal>>= Vec::new();
    for loc in sb_locs {
        info!("\tVerifying superblock at {}",loc);
        let sb = crate::Superblock::read(d.clone(), loc).ok();
        if let Some(sb) = sb {
            geom_locs.push((0..16).map(|i| sb.geometries(i)).collect());
            info!("\t\tOK!");
        } else {
            warn!("\t\tNot OK");
            let mut sb = unsafe { crate::Superblock::read_unchecked(d.clone(), loc)? } ;
            if sb.signature() != SIGNATURE {
                warn!("\t\t\tIncorrect signature");
            }
            if !sb.verify_checksum() {
                warn!("\t\t\tIncorrect checksum");
            }
            if let Some(fs) = &fs {
                if sb.devid() != fs.read().unwrap().get_superblock().unwrap().devid() {
                    warn!("\t\t\tMismatched device ID");
                }
            }
        }
    }
    unimplemented!();
}