use std::sync::{Arc,RwLock};
use std::collections::{BTreeSet,BTreeMap};
use crate::{Superblock,Disk,DiskGroup,Allocator,FSGroup,ObjectSet,AMPointerGlobal};
use amos_std::AMResult;
use amos_std::error::AMErrorFS;
use std::convert::TryInto;

/// Object used for mounting a filesystem
#[derive(Debug)]
pub struct AMFS {
    dgs: [Option<DiskGroup>;16],
    superblocks: BTreeMap<u64,[Option<Superblock>;4]>,
    allocators: BTreeMap<u64,Allocator>,
    lock: Arc<RwLock<u8>>,
}

impl AMFS {
    /// Creates an AMFS object to mount the fs on a disk
    #[cfg(feature="unstable")]
    pub fn open(d: &[Disk]) -> AMResult<AMFS> {
        let mut res = AMFS{dgs:Default::default(),superblocks:BTreeMap::new(),allocators:BTreeMap::new(),lock: Arc::new(RwLock::new(0))};
        let devids = res.load_superblocks(d)?;
        res.build_diskgroups(&devids,d)?;
        res.load_allocators()?;
        assert!(res.test_features(crate::AMFeatures::current_set())?);
        Ok(res)
    }
    #[cfg(feature="stable")]
    fn test_features(&self, features: BTreeSet<usize>) -> AMResult<bool> {
        Ok(self.get_superblock()?.test_features(features))
    }
    /*fn get_geometry(&self, n: u8) -> Result<Geometry,u8> {
        Ok(self.dg[n as usize].as_ref().unwrap().geo)
    }*/
    #[cfg(feature="stable")]
    fn get_superblock(&self) -> AMResult<Superblock> {
        Ok(
            self.superblocks.values().flatten().filter_map(|x| *x).fold(
                None,
                |acc:Option<(u128,Superblock)>,x| {
                    if let Some((max,_)) = acc {
                        if let Ok(group) = x.get_group(&self.dgs) {
                            if group.get_txid()>max {
                                Some((group.get_txid(),x))
                            } else {
                                acc
                            }
                        } else {
                            acc
                        }
                    } else {
                        if let Ok(group) = x.get_group(&self.dgs) {
                            Some((group.get_txid(),x))
                        } else {
                            acc
                        }
                    }
                }
            ).ok_or(AMErrorFS::NoSuperblock)?.1
        )
    }
    #[cfg(feature="stable")]
    fn get_root_group(&self) -> AMResult<FSGroup> {
        self.get_superblock()?.get_group(&self.dgs)
    }
    #[cfg(feature="stable")]
    fn load_superblocks(&mut self, ds: &[Disk]) -> AMResult<Vec<u64>> {
        let mut res = Vec::with_capacity(ds.len());
        for d in ds {
            let mut disk_devid = None;
            let sb_locs = d.get_header_locs()?;
            for (i,loc) in sb_locs.iter().enumerate() {
                if let Ok(hdr) = Superblock::read(d.clone(),*loc) {
                    let devid = hdr.devid();
                    info!("Superblock {:x}:{} OK",devid,i);
                    self.superblocks.entry(devid).or_insert([None;4])[i]=Some(hdr);
                    disk_devid = Some(devid);
                } else {
                    warn!("Superblock ?:{} corrupted",i);
                }
            }
            res.push(disk_devid.ok_or(AMErrorFS::NoSuperblock)?);
        }
        Ok(res)
    }
    #[cfg(feature="stable")]
    fn build_diskgroups(&mut self, devids: &[u64], ds: &[Disk]) -> AMResult<()> {
        for (devid,sbs) in self.superblocks.iter() {
            let diskno = devids.iter().position(|r| r == devid).expect("Superblock with devid matching no disk");
            for (sbn,sbo) in sbs.iter().enumerate() {
                if let Some(sb) = sbo {
                    for i in 0..16 {
                        if self.dgs[i].is_none() {
                            if !sb.geometries[i].is_null() {
                                if let Ok(geo) = sb.get_geometry(ds[diskno].clone(),i.try_into().or(Err(0))?){
                                    info!("Built diskgroup using {:x}:{}:{}",devid,sbn,i);
                                    self.dgs[i]=Some(DiskGroup::from_geo(geo,devids,ds));
                                } else {
                                    error!("Corrupt geometry: {:x}:{}:{}",devid,sbn,i);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    #[cfg(feature="stable")]
    fn load_allocators(&mut self) -> AMResult<()> {
        self.allocators = self.get_superblock()?.get_group(&self.dgs)?.get_allocators(&self.dgs)?;
        for dg in self.dgs.iter_mut().flatten() {
                dg.load_allocators(self.allocators.clone());
        }
        Ok(())
    }
    /// Allocates a number of blocks in the filesystem
    #[cfg(feature="unstable")]
    pub fn alloc(&self, n: u64) -> AMResult<Option<AMPointerGlobal>> {
        Ok(Some(self.dgs[0].clone().ok_or(0)?.alloc(n)?))
    }
    /// Gets the filesystem's object tree
    #[cfg(feature="unstable")]
    pub fn get_objects(&self) -> AMResult<ObjectSet> {
        let obj_ptr = self.get_root_group()?.get_obj_ptr();
        ObjectSet::read(self.dgs.clone(),obj_ptr)
    }
    /// Reads the object corresponding to a given ID
    #[cfg(feature="stable")]
    pub fn read_object(&self, id: u64,start:u64,data:&mut [u8]) -> AMResult<u64> {
        self.get_objects()?.get_object(id)?.ok_or(0)?.read(start,data,&self.dgs)
    }
    /// Writes to the object corresponding to a given ID
    #[cfg(feature="unstable")]
    pub fn write_object(&self, id: u64,start:u64,data:&[u8]) -> AMResult<u64> {
        self.get_objects()?.get_object(id)?.ok_or(0)?.write(start,data,&self.dgs)
    }
    /// Writes to the object corresponding to a given ID
    #[cfg(feature="stable")]
    pub fn size_object(&self, id: u64) -> AMResult<u64> {
        self.get_objects()?.get_object(id)?.ok_or(0)?.size()
    }
    /// Syncs the disks
    #[cfg(feature="stable")]
    pub fn sync(&mut self) -> AMResult<()> {
        for i in &mut self.dgs {
            if let Some(dg) = i {
                dg.sync()?
            }
        }
        Ok(())
    }
}