use std::collections::BTreeMap;

use crate::{Superblock,Geometry,Allocator,Disk,AMPointerLocal,DiskGroup,FSGroup};
use crate::BLOCK_SIZE;

use amos_std::AMResult;

/// Makes a new AMFS filesystem composed of a single disk.
#[cfg(feature = "unstable")]
pub fn mkfs_single(mut d: Disk) -> AMResult<()> {
    //Erase disk
        let dsize = d.size()?;
        for i in 0..dsize {
            d.write_at(i,&[0;BLOCK_SIZE])?;
        }
    //Generate device ID
        let devid = rand::random::<u64>();
    //Calculate header locations
        let header_locs = d.get_header_locs()?;
    //Create free block map, mark headers used.
        let mut free = Allocator::new(d.size()?);
        for loc in header_locs {
            free.mark_used(loc.loc(),1)?;
        }

    let mut sbs = [Superblock::new(devid);4];

    //Create geometries
        let mut geom = Geometry::new();
        geom.device_ids[0] = devid;
        for sb in &mut sbs {
            //Create geometry
            let geo_ptr = free.alloc(1).ok_or(0)?;
            let geo_ptr = geom.write(d.clone(),AMPointerLocal::new(geo_ptr))?;

            sb.geometries[0]=geo_ptr;
        }
    //Create disk group
        let mut dg = DiskGroup::single(geom,d.clone(),free.clone());
    //Write root group
        let mut root_group = FSGroup::new();
        root_group.objects = dg.alloc(1)?;
        let mut amap = BTreeMap::new();
        amap.insert(devid,free);
        let mut root_ptr = dg.alloc(1)?;
        root_group.write_allocators(&mut [Some(dg.clone())], &mut amap)?;
        root_group.write(&[Some(dg)],&mut root_ptr)?;
        for sb in &mut sbs {
            sb.rootnodes[0] = root_ptr;
            sb.latest_root = 0;
        }
    //Write superblocks
    for i in 0..4 {
        sbs[i].write(d.clone(),header_locs[i])?;
    }
    //Sync disk
        d.sync()?;
    Ok(())
}

#[test]
pub fn test_mkfs() {
    #![allow(clippy::unwrap_used)]
    let d = crate::DiskMem::open(1000);
    mkfs_single(d).unwrap();
}