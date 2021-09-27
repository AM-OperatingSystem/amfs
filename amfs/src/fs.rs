use crate::features::AMFeatures;
use crate::{
    AMPointerGlobal, Allocator, Disk, DiskGroup, FSGroup, Fragment, JournalEntry, Object,
    ObjectSet, Superblock,
};
use amos_std::error::AMErrorFS;
use amos_std::AMResult;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::convert::TryInto;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// A handle to a disk
#[derive(Clone, Debug)]
pub struct FSHandle(Arc<RwLock<AMFS>>);

impl FSHandle {
    /// Creates an AMFS object to mount the fs on a disk
    pub fn open(d: &[Disk]) -> AMResult<Self> {
        Ok(Self {
            0: Arc::new(RwLock::new(AMFS::open(d)?)),
        })
    }
    /// Write changes to disk
    #[cfg(feature = "unstable")]
    pub fn commit(&self) -> AMResult<()> {
        self.write()?.commit()
    }
    /// Reads the object corresponding to a given ID
    #[cfg(feature = "stable")]
    pub fn read_object(&self, id: u64, start: u64, data: &mut [u8]) -> AMResult<u64> {
        self.read()?.read_object(id, start, data)
    }
    /// Gets the size of the object corresponding to a given ID
    #[cfg(feature = "stable")]
    pub fn size_object(&self, id: u64) -> AMResult<u64> {
        self.read()?.size_object(id)
    }
    /// Writes to the object corresponding to a given ID
    #[cfg(feature = "unstable")]
    pub fn write_object(&self, id: u64, start: u64, data: &[u8]) -> AMResult<u64> {
        self.write()?.write_object(id, start, data)
    }
    /// Writes to the object corresponding to a given ID
    #[cfg(feature = "unstable")]
    pub fn create_object(&self, id: u64, size: u64) -> AMResult<()> {
        self.write()?.create_object(id, size)
    }
    /// Truncates the object corresponding to a given ID
    #[cfg(feature = "unstable")]
    pub fn truncate_object(&self, id: u64, size: u64) -> AMResult<()> {
        self.write()?.truncate_object(id, size)
    }
    /// Syncs the disks
    #[cfg(feature = "stable")]
    pub fn sync(&self) -> AMResult<()> {
        self.write()?.sync()
    }
    /// Allocates a n-block chunk
    pub(crate) fn alloc_blocks(&mut self, n: u64) -> AMResult<Option<AMPointerGlobal>> {
        self.write()?.alloc_blocks(n)
    }
    /// Reallocates a pointer
    pub(crate) fn realloc(&mut self, ptr: AMPointerGlobal) -> AMResult<Option<AMPointerGlobal>> {
        self.write()?.realloc(ptr)
    }
    /// Frees a pointer
    pub(crate) fn free(&mut self, ptr: AMPointerGlobal) -> AMResult<()> {
        self.write()?.free(ptr)
    }
    pub(crate) fn write(&self) -> AMResult<RwLockWriteGuard<AMFS>> {
        Ok(self.0.write()?)
    }
    pub(crate) fn read(&self) -> AMResult<RwLockReadGuard<AMFS>> {
        Ok(self.0.read()?)
    }
}

/// Object used for mounting a filesystem
#[derive(Debug)]
pub struct AMFS {
    dgs: [Option<DiskGroup>; 16],
    disks: BTreeMap<u64, Disk>,
    diskids: BTreeSet<u64>,
    superblocks: BTreeMap<u64, [Option<Superblock>; 4]>,
    allocators: BTreeMap<u64, Allocator>,
    lock: Arc<RwLock<u8>>,
    journal: VecDeque<JournalEntry>,
    objects: Option<ObjectSet>,
}

impl AMFS {
    #[cfg(feature = "unstable")]
    fn open(d: &[Disk]) -> AMResult<AMFS> {
        let mut res = AMFS {
            dgs: Default::default(),
            disks: BTreeMap::new(),
            diskids: BTreeSet::new(),
            superblocks: BTreeMap::new(),
            allocators: BTreeMap::new(),
            lock: Arc::new(RwLock::new(0)),
            journal: VecDeque::new(),
            objects: None,
        };
        let devids = res.load_superblocks(d)?;
        res.build_diskgroups(&devids, d)?;
        res.load_allocators()?;
        assert!(res.test_features(AMFeatures::current_set())?);
        let obj_ptr = res.get_root_group()?.get_obj_ptr();
        res.objects = Some(ObjectSet::read(res.dgs.clone(), obj_ptr)?);
        Ok(res)
    }
    #[cfg(feature = "stable")]
    fn test_features(&self, features: BTreeSet<usize>) -> AMResult<bool> {
        Ok(self.get_superblock()?.test_features(features))
    }
    #[cfg(feature = "stable")]
    fn get_superblock(&self) -> AMResult<Superblock> {
        Ok(self
            .superblocks
            .values()
            .flatten()
            .filter_map(|x| *x)
            .fold(None, |acc: Option<(u128, Superblock)>, x| {
                if let Some((max, _)) = acc {
                    if let Ok(group) = x.get_group(&self.dgs) {
                        if group.get_txid() > max {
                            Some((group.get_txid(), x))
                        } else {
                            acc
                        }
                    } else {
                        acc
                    }
                } else {
                    if let Ok(group) = x.get_group(&self.dgs) {
                        Some((group.get_txid(), x))
                    } else {
                        acc
                    }
                }
            })
            .ok_or(AMErrorFS::NoSuperblock)?
            .1)
    }
    #[cfg(feature = "stable")]
    fn get_root_group(&self) -> AMResult<FSGroup> {
        self.get_superblock()?.get_group(&self.dgs)
    }
    #[cfg(feature = "stable")]
    fn load_superblocks(&mut self, ds: &[Disk]) -> AMResult<Vec<u64>> {
        let mut res = Vec::with_capacity(ds.len());
        for d in ds {
            let mut disk_devid = None;
            let sb_locs = d.get_header_locs()?;
            for (i, loc) in sb_locs.iter().enumerate() {
                if let Ok(hdr) = Superblock::read(d.clone(), *loc) {
                    let devid = hdr.devid();
                    info!("Superblock {:x}:{} OK", devid, i);
                    self.superblocks.entry(devid).or_insert([None; 4])[i] = Some(hdr);
                    self.disks.entry(devid).or_insert_with(|| d.clone());
                    self.diskids.insert(devid);
                    disk_devid = Some(devid);
                } else {
                    warn!("Superblock ?:{} corrupted", i);
                }
            }
            res.push(disk_devid.ok_or(AMErrorFS::NoSuperblock)?);
        }
        Ok(res)
    }
    #[cfg(feature = "stable")]
    fn build_diskgroups(&mut self, devids: &[u64], ds: &[Disk]) -> AMResult<()> {
        for (devid, sbs) in self.superblocks.iter() {
            let diskno = devids
                .iter()
                .position(|r| r == devid)
                .expect("Superblock with devid matching no disk");
            for (sbn, sbo) in sbs.iter().enumerate() {
                if let Some(sb) = sbo {
                    for i in 0..16 {
                        if self.dgs[i].is_none() {
                            if !sb.geometries[i].is_null() {
                                if let Ok(geo) =
                                    sb.get_geometry(ds[diskno].clone(), i.try_into().or(Err(0))?)
                                {
                                    info!("Built diskgroup using {:x}:{}:{}", devid, sbn, i);
                                    self.dgs[i] = Some(DiskGroup::from_geo(geo, devids, ds));
                                } else {
                                    error!("Corrupt geometry: {:x}:{}:{}", devid, sbn, i);
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
    #[cfg(feature = "stable")]
    fn load_allocators(&mut self) -> AMResult<()> {
        self.allocators = self
            .get_superblock()?
            .get_group(&self.dgs)?
            .get_allocators(&self.dgs)?;
        for dg in self.dgs.iter_mut().flatten() {
            dg.load_allocators(self.allocators.clone());
        }
        Ok(())
    }
    #[cfg(feature = "unstable")]
    pub(crate) fn alloc_blocks(&mut self, n: u64) -> AMResult<Option<AMPointerGlobal>> {
        let lock = self.lock.clone();
        let _handle = lock.read()?;

        let res = self.dgs[0].clone().ok_or(0)?.alloc_blocks(n)?;
        self.journal.push_back(JournalEntry::Alloc(res));

        Ok(Some(res))
    }
    #[cfg(feature = "unstable")]
    pub(crate) fn alloc_bytes(&mut self, n: u64) -> AMResult<Vec<Fragment>> {
        let lock = self.lock.clone();
        let _handle = lock.read()?;

        let res = self.dgs[0].clone().ok_or(0)?.alloc_bytes(n)?;
        //TODO: self.journal.push_back(JournalEntry::Alloc(res));

        Ok(res)
    }
    #[cfg(feature = "unstable")]
    pub(crate) fn realloc(&mut self, ptr: AMPointerGlobal) -> AMResult<Option<AMPointerGlobal>> {
        let lock = self.lock.clone();
        let _handle = lock.read()?;

        let n = ptr.length();
        let new_ptr = if let Some(p) = self.alloc_blocks(n.into())? {
            p
        } else {
            return Ok(None);
        };
        let contents = ptr.read_vec(&self.dgs)?;
        new_ptr.write(0, contents.len(), &self.dgs, &contents)?;
        self.free(ptr)?;
        Ok(Some(new_ptr))
    }
    #[cfg(feature = "unstable")]
    pub(crate) fn free(&mut self, ptr: AMPointerGlobal) -> AMResult<()> {
        let lock = self.lock.clone();
        let _handle = lock.read()?;

        self.journal.push_back(JournalEntry::Free(ptr));

        Ok(())
    }
    #[cfg(feature = "unstable")]
    pub(crate) fn get_objects(&self) -> AMResult<&ObjectSet> {
        Ok(self.objects.as_ref().expect("PANIC"))
    }
    #[cfg(feature = "unstable")]
    pub(crate) fn get_objects_mut(&mut self) -> AMResult<&mut ObjectSet> {
        Ok(self.objects.as_mut().expect("PANIC"))
    }
    #[cfg(feature = "stable")]
    fn read_object(&self, id: u64, start: u64, data: &mut [u8]) -> AMResult<u64> {
        self.get_objects()?.read_object(id, start, data, &self.dgs)
    }
    /// Gets the size of the object corresponding to a given ID
    #[cfg(feature = "stable")]
    fn size_object(&self, id: u64) -> AMResult<u64> {
        self.get_objects()?.size_object(id)
    }
    /// Truncates the object corresponding to a given ID
    #[cfg(feature = "stable")]
    fn truncate_object(&mut self, id: u64, len: u64) -> AMResult<()> {
        assert!(self.get_objects()?.exists_object(id)?);
        let dgs = &self.dgs.clone();
        let mut obj = self.get_objects()?.get_object(id)?.ok_or(0)?;
        obj.truncate(self, len, dgs)?;
        let objs = self.get_objects()?.clone();
        let objs = objs.set_object(self, id, obj)?;
        *self.get_objects_mut()? = objs;
        Ok(())
    }
    /// Writes to the object corresponding to a given ID
    #[cfg(feature = "unstable")]
    fn write_object(&mut self, id: u64, start: u64, data: &[u8]) -> AMResult<u64> {
        let dgs = &self.dgs.clone();
        let mut obj = self.get_objects()?.get_object(id)?.ok_or(0)?;
        let res = obj.write(self, start, data, dgs)?;
        let objs = self.get_objects()?.clone();
        let objs = objs.set_object(self, id, obj)?;
        *self.get_objects_mut()? = objs;
        Ok(res)
    }
    /// Writes to the object corresponding to a given ID
    #[cfg(feature = "unstable")]
    fn create_object(&mut self, id: u64, size: u64) -> AMResult<()> {
        let ptr = self.alloc_blocks(1)?.ok_or(0)?;
        let frag = Fragment::new(size, 0, ptr);
        let obj = Object::new(&[frag]);
        let objs = self.get_objects()?.clone();
        let objs = objs.set_object(self, id, obj)?;
        *self.get_objects_mut()? = objs;
        Ok(())
    }
    /// Syncs the disks
    #[cfg(feature = "stable")]
    fn sync(&mut self) -> AMResult<()> {
        for i in &mut self.dgs {
            if let Some(dg) = i {
                dg.sync()?
            }
        }
        Ok(())
    }
    fn commit(&mut self) -> AMResult<()> {
        let lock = self.lock.clone();
        let _handle = lock.write()?;
        let mut dg = self.dgs[0].clone().ok_or(0)?;
        let mut root_group = self.get_root_group()?;
        root_group.objects = self.get_objects()?.ptr;
        let mut root_ptr = dg.alloc_blocks(1)?;
        root_group.write_allocators(&mut [Some(dg.clone())], &mut self.allocators)?;
        root_group.write(&[Some(dg)], &mut root_ptr)?;
        // Write superblocks
        for disk_id in &self.diskids {
            for i in 0..4 {
                if let Some(sb) = &mut self.superblocks.get_mut(disk_id).ok_or(0)?[i] {
                    sb.latest_root += 1;
                    sb.rootnodes[usize::from(sb.latest_root)] = root_ptr;
                    let header_locs = self.disks[disk_id].get_header_locs()?;
                    sb.write(self.disks[disk_id].clone(), header_locs[i])?;
                }
            }
        }
        self.sync()?;
        Ok(())
    }
}
