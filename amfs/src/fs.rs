use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    convert::TryInto,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use amos_std::{
    error::{AMError, AMErrorFS},
    AMResult,
};

use crate::{
    features::AMFeatures, AMPointerGlobal, Allocator, Disk, DiskGroup, FSGroup, Fragment,
    JournalEntry, Object, ObjectSet, Superblock,
};

/// A handle to a disk
#[derive(Clone, Debug)]
pub struct FSHandle(Arc<RwLock<AMFS>>);

impl FSHandle {
    /// Creates an AMFS object to mount the fs on a disk
    #[cfg(feature = "unstable")]
    pub fn open(d: &[Disk]) -> AMResult<Self> {
        Ok(Self(Arc::new(RwLock::new(AMFS::open(d)?))))
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
    #[cfg(feature = "stable")]
    pub(crate) fn alloc_blocks(&mut self, n: u64) -> AMResult<Option<AMPointerGlobal>> {
        self.write()?.alloc_blocks(n)
    }
    /// Reallocates a pointer
    #[cfg(feature = "stable")]
    pub(crate) fn realloc(&mut self, ptr: AMPointerGlobal) -> AMResult<Option<AMPointerGlobal>> {
        self.write()?.realloc(ptr)
    }
    /// Frees a pointer
    #[cfg(feature = "stable")]
    pub(crate) fn free(&mut self, ptr: AMPointerGlobal) -> AMResult<()> {
        self.write()?.free(ptr)
    }
    #[cfg(feature = "stable")]
    pub(crate) fn write(&self) -> AMResult<RwLockWriteGuard<AMFS>> {
        Ok(self.0.write().or(Err(AMError::Poison))?)
    }
    #[cfg(feature = "stable")]
    pub(crate) fn read(&self) -> AMResult<RwLockReadGuard<AMFS>> {
        Ok(self.0.read().or(Err(AMError::Poison))?)
    }
}

/// Object used for mounting a filesystem
#[derive(Debug)]
pub struct AMFS {
    diskgroups:  Vec<Option<DiskGroup>>,
    disks:       BTreeMap<u64, Disk>,
    diskids:     BTreeSet<u64>,
    superblocks: BTreeMap<u64, [Option<Superblock>; 4]>,
    allocators:  BTreeMap<u64, Allocator>,
    lock:        Arc<RwLock<u8>>,
    journal:     VecDeque<JournalEntry>,
    objects:     Option<ObjectSet>,
    free_queue:  BTreeMap<u128, Vec<AMPointerGlobal>>,
    cur_txid:    u128,
}

impl AMFS {
    #[cfg(feature = "unstable")]
    fn open(d: &[Disk]) -> AMResult<AMFS> {
        let mut res = AMFS {
            diskgroups:  vec![None; 16],
            disks:       BTreeMap::new(),
            diskids:     BTreeSet::new(),
            superblocks: BTreeMap::new(),
            allocators:  BTreeMap::new(),
            lock:        Arc::new(RwLock::new(0)),
            journal:     VecDeque::new(),
            objects:     None,
            free_queue:  BTreeMap::new(),
            cur_txid:    0,
        };
        let devids = res.load_superblocks(d)?;
        res.build_diskgroups(&devids, d)?;
        res.load_allocators()?;
        assert!(res.test_features(AMFeatures::current_set())?);
        let obj_ptr = res.get_root_group()?.get_obj_ptr();
        res.objects = Some(ObjectSet::read(res.diskgroups.clone(), obj_ptr));
        res.cur_txid = res.get_root_group()?.txid() + 1;
        Ok(res)
    }
    #[cfg(feature = "stable")]
    fn test_features(&self, features: BTreeSet<usize>) -> AMResult<bool> {
        Ok(self.get_superblock()?.test_features(features))
    }
    #[cfg(feature = "stable")]
    pub(crate) fn get_superblock(&self) -> AMResult<Superblock> {
        Ok(self
            .superblocks
            .values()
            .flatten()
            .filter_map(|x| *x)
            .fold(None, |acc: Option<(u128, Superblock)>, x| {
                if let Some((max, _)) = acc {
                    if let Ok(group) = x.get_group(&self.diskgroups) {
                        if group.txid() > max {
                            Some((group.txid(), x))
                        } else {
                            acc
                        }
                    } else {
                        acc
                    }
                } else {
                    if let Ok(group) = x.get_group(&self.diskgroups) {
                        Some((group.txid(), x))
                    } else {
                        acc
                    }
                }
            })
            .ok_or(AMErrorFS::NoFSGroup)?
            .1)
    }
    #[cfg(feature = "stable")]
    fn get_root_group(&self) -> AMResult<FSGroup> {
        self.get_superblock()?.get_group(&self.diskgroups)
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
        for (devid, superblocks) in self.superblocks.iter() {
            let disk_no = devids
                .iter()
                .position(|r| r == devid)
                .ok_or(AMErrorFS::UnknownDevId)?;
            for (sbn, sbo) in superblocks.iter().enumerate() {
                if let Some(sb) = sbo {
                    for i in 0..16 {
                        if self.diskgroups[i].is_none() {
                            if !sb.geometries[i].is_null() {
                                if let Ok(geo) = sb.get_geometry(
                                    ds[disk_no].clone(),
                                    i.try_into().or(Err(AMErrorFS::NoDiskgroup))?,
                                ) {
                                    info!("Built diskgroup using {:x}:{}:{}", devid, sbn, i);
                                    self.diskgroups[i] =
                                        Some(DiskGroup::from_geo(geo, devids, ds)?);
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
            .get_group(&self.diskgroups)?
            .get_allocators(&self.diskgroups)?;
        for dg in self.diskgroups.iter_mut().flatten() {
            dg.load_allocators(self.allocators.clone())?;
        }
        self.free_queue = self
            .get_superblock()?
            .get_group(&self.diskgroups)?
            .get_free_queue(&self.diskgroups)?;
        Ok(())
    }
    #[cfg(feature = "unstable")]
    pub(crate) fn alloc_blocks(&mut self, n: u64) -> AMResult<Option<AMPointerGlobal>> {
        let lock = self.lock.clone();
        let _handle = lock.read().or(Err(AMError::Poison))?;

        let mut res = self.diskgroups[0]
            .clone()
            .ok_or(AMErrorFS::NoDiskgroup)?
            .alloc_blocks(n)?;
        res.update(&self.diskgroups)?;
        self.journal.push_back(JournalEntry::Alloc(res));

        Ok(Some(res))
    }
    #[cfg(feature = "unstable")]
    pub(crate) fn alloc_bytes(&mut self, n: u64) -> AMResult<Vec<Fragment>> {
        let lock = self.lock.clone();
        let _handle = lock.read().or(Err(AMError::Poison))?;

        let mut res = self.diskgroups[0]
            .clone()
            .ok_or(AMError::TODO(0))?
            .alloc_bytes(n)?;
        for p in &mut res {
            p.pointer.update(&self.diskgroups)?;
        }
        //TODO: self.journal.push_back(JournalEntry::Alloc(res));

        Ok(res)
    }
    #[cfg(feature = "unstable")]
    pub(crate) fn realloc(&mut self, ptr: AMPointerGlobal) -> AMResult<Option<AMPointerGlobal>> {
        let lock = self.lock.clone();
        let _handle = lock.read().or(Err(AMError::Poison))?;

        let n = ptr.length();
        let new_ptr = if let Some(p) = self.alloc_blocks(n.into())? {
            p
        } else {
            return Ok(None);
        };
        let contents = ptr.read_vec(&self.diskgroups)?;
        new_ptr.write(0, contents.len(), &self.diskgroups, &contents)?;
        self.free(ptr)?;
        Ok(Some(new_ptr))
    }
    #[cfg(feature = "unstable")]
    pub(crate) fn free(&mut self, ptr: AMPointerGlobal) -> AMResult<()> {
        info!("Freeing {}", ptr);
        let lock = self.lock.clone();
        let _handle = lock.read().or(Err(AMError::Poison))?;

        self.journal.push_back(JournalEntry::Free(ptr));
        if let Some(e) = self.free_queue.get_mut(&self.cur_txid) {
            e.push(ptr);
        } else {
            self.free_queue.insert(self.cur_txid, vec![ptr]);
        }

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
        self.get_objects()?
            .read_object(id, start, data, &self.diskgroups)
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
        let diskgroups = &self.diskgroups.clone();
        let mut obj = self
            .get_objects()?
            .get_object(id)?
            .ok_or(AMErrorFS::NoObject)?;
        obj.truncate(self, len, diskgroups)?;
        let objs = self.get_objects()?.clone();
        let objs = objs.set_object(self, id, obj)?;
        *self.get_objects_mut()? = objs;
        Ok(())
    }
    /// Writes to the object corresponding to a given ID
    #[cfg(feature = "unstable")]
    fn write_object(&mut self, id: u64, start: u64, data: &[u8]) -> AMResult<u64> {
        let diskgroups = &self.diskgroups.clone();
        let mut obj = self
            .get_objects()?
            .get_object(id)?
            .ok_or(AMErrorFS::NoObject)?;
        let res = obj.write(self, start, data, diskgroups)?;
        let objs = self.get_objects()?.clone();
        let objs = objs.set_object(self, id, obj)?;
        *self.get_objects_mut()? = objs;
        Ok(res)
    }
    /// Writes to the object corresponding to a given ID
    #[cfg(feature = "unstable")]
    fn create_object(&mut self, id: u64, size: u64) -> AMResult<()> {
        let ptr = self.alloc_blocks(1)?.ok_or(AMError::TODO(0))?;
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
        for i in &mut self.diskgroups {
            if let Some(dg) = i {
                dg.sync()?
            }
        }
        Ok(())
    }
    #[cfg(feature = "unstable")]
    fn commit(&mut self) -> AMResult<()> {
        let lock = self.lock.clone();
        let _handle = lock.write().or(Err(AMError::Poison))?;
        let mut dg = self.diskgroups[0].clone().ok_or(AMErrorFS::NoDiskgroup)?;
        let mut root_group = self.get_root_group()?;
        root_group.objects = self.get_objects()?.ptr;
        let mut root_ptr = dg.alloc_blocks(1)?;
        root_group.write_free_queue(&[Some(dg.clone())], &self.free_queue)?;
        root_group.write_allocators(&mut [Some(dg.clone())], &mut self.allocators)?;
        root_group.write(&[Some(dg)], &mut root_ptr)?;
        // Write superblocks
        for disk_id in &self.diskids {
            for i in 0..4 {
                if let Some(sb) = &mut self.superblocks.get_mut(disk_id).ok_or(AMError::TODO(0))?[i]
                {
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
