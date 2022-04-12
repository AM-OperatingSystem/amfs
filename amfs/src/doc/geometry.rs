//!
//! AMFS stores [geometry objects](crate::Geometry) to allow reconfiguration of the array while online.
//!
//! In order to modify the shape of the array while the filesystem is online,
//! a layer of abstraction is necessary between block pointers and the on-disk location of the referenced data.
//!
//! This is done using the geometry table.
//!
//! Each pointer stores the index into the geometry table of the geometry to be used when calculating its location on disk.
//!
//! When changing geometries, the new geometry is stored into a free slot in the geometry table,
//! and the active geometry is updated accordingly.
//!
//! Newly allocated blocks are now stored according to the new geometry.
//!
//! If we are migrating away from the old geometry, blocks are rewritten in the background to match the new geometry.
//!
//! Once all old blocks are rewritten, the old geometry is removed from the geometry table, and any disks present only in the old geometry can be removed.
