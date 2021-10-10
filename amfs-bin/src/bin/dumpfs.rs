#![cfg(not(tarpaulin_include))]
#![allow(clippy::all)]
#![allow(unknown_lints)]
#![allow(require_stability_comment)]

use amfs::*;

use amfs::{BLOCK_SIZE, SIGNATURE};

use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

use strum::IntoEnumIterator;

use colored::*;

#[repr(C)]
pub(crate) struct LLGHeader {
    next: AMPointerGlobal,
    count: u16,
    _padding: u64,
}

#[derive(Debug, Clone)]
enum BlockType {
    Unused,
    Superblock(Superblock),
    Geometry(Geometry),
    FSGroup(FSGroup),
    Alloc(AMPointerGlobal),
    AllocList(AMPointerGlobal),
    FreeQueue(AMPointerGlobal),
    Objects(ObjectSet),
    Error,
}

fn main() {
    let path = std::env::args().nth(1).unwrap();
    let mut d = DiskFile::open(&path).unwrap();
    let mut dg = DiskGroup::single(Geometry::new(), d.clone(), Allocator::new(0));
    println!("Image is {} blocks long", d.size().unwrap());
    let sb_locs = d.get_header_locs().unwrap();
    let mut types = vec![(BlockType::Unused, false); d.size().unwrap().try_into().unwrap()];
    print!("Header locations:");
    for loc in sb_locs {
        print!("{} ", loc.loc());
        unsafe {
            types[usize::try_from(loc.loc()).unwrap()] = (
                BlockType::Superblock(Superblock::read_unchecked(d.clone(), loc).unwrap()),
                false,
            );
        }
    }
    println!();
    loop {
        let mut upd = false;
        for (idx, typ) in types.clone().iter().enumerate() {
            if typ.1 {
                continue;
            }
            match &typ.0 {
                BlockType::Unused => continue,
                BlockType::Error => continue,
                BlockType::Superblock(s) => {
                    dg.geo.device_ids[0] = s.devid();
                    for i in 0..16 {
                        if s.geometries(i).is_null() {
                            continue;
                        }
                        if let Ok(g) = Geometry::read(d.clone(), s.geometries(i)) {
                            types[s.geometries(i).loc() as usize] = (BlockType::Geometry(g), false)
                        } else {
                            types[s.geometries(i).loc() as usize] = (BlockType::Error, true)
                        }
                    }
                    for i in 0..128 {
                        if s.rootnodes(i).is_null() {
                            continue;
                        }
                        if let Ok(g) = FSGroup::read(&[Some(dg.clone())], s.rootnodes(i)) {
                            types[s.rootnodes(i).loc() as usize] = (BlockType::FSGroup(g), false)
                        } else {
                            types[s.rootnodes(i).loc() as usize] = (BlockType::Error, true)
                        }
                    }
                    types[idx].1 = true;
                    upd = true;
                }
                BlockType::Geometry(_) => {
                    types[idx].1 = true;
                    upd = true;
                }
                BlockType::AllocList(a) => {
                    let mut buf = [0u8; BLOCK_SIZE];
                    a.read(0, BLOCK_SIZE, &[Some(dg.clone())], &mut buf)
                        .unwrap();
                    let hdr = unsafe { u8_slice_as_any::<LLGHeader>(&buf) };
                    if !hdr.next.is_null() {
                        types[hdr.next.loc() as usize] = (BlockType::AllocList(hdr.next), false)
                    }
                    for i in 0..usize::from(hdr.count) {
                        let ptr = unsafe {
                            u8_slice_as_any::<AMPointerGlobal>(&buf[0x28 + i * 24..0x38 + i * 24])
                        };
                        types[ptr.loc() as usize] = (BlockType::Alloc(*ptr), false)
                    }
                    types[idx].1 = true;
                    upd = true;
                }
                BlockType::Alloc(_) => {
                    types[idx].1 = true;
                    upd = true;
                }
                BlockType::Objects(_) => {
                    types[idx].1 = true;
                    upd = true;
                }
                BlockType::FreeQueue(_) => {
                    types[idx].1 = true;
                    upd = true;
                }
                BlockType::FSGroup(f) => {
                    if !f.alloc().is_null() {
                        types[f.alloc().loc() as usize] = (BlockType::AllocList(f.alloc()), false)
                    }
                    if !f.objects().is_null() {
                        types[f.objects().loc() as usize] = (
                            BlockType::Objects(ObjectSet::read(
                                [
                                    Some(dg.clone()),
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
                                f.objects(),
                            )),
                            false,
                        )
                    }
                    if !f.free_queue().is_null() {
                        types[f.free_queue().loc() as usize] =
                            (BlockType::FreeQueue(f.free_queue()), false)
                    }
                    types[idx].1 = true;
                    upd = true;
                }
            }
        }
        if !upd {
            break;
        }
    }
    let mut buf = [0; BLOCK_SIZE];
    for (idx, typ) in types.iter().enumerate() {
        d.read_at(idx.try_into().unwrap(), &mut buf).unwrap();
        match typ.0.clone() {
            BlockType::Unused => print_unused(idx, buf),
            BlockType::Superblock(s) => print_superblock(idx, buf, s, &d),
            BlockType::Geometry(g) => print_geometry(idx, buf, g, &d),
            BlockType::FSGroup(f) => print_fsgroup(idx, buf, f, &dg),
            BlockType::AllocList(_) => print_alloclist(idx, buf),
            BlockType::Alloc(_) => print_alloc(idx, buf),
            BlockType::Objects(o) => print_objs(idx, buf, o, &dg),
            BlockType::FreeQueue(_) => print_free_queue(idx, buf),
            BlockType::Error => print_error(idx, buf),
        }
    }
}

fn print_unused(_idx: usize, _buf: [u8; BLOCK_SIZE]) {
    //println!("Unused");
}
fn print_fsgroup(idx: usize, buf: [u8; BLOCK_SIZE], g: FSGroup, _d: &DiskGroup) {
    println!("FSGroup:");
    print_hex_ptr_global(
        idx * BLOCK_SIZE + 0,
        &buf[0x10 * (0)..],
        "alloc".to_string(),
        g.alloc(),
    );
    println!();
    print_hex_ptr_global(
        idx * BLOCK_SIZE + 1,
        &buf[0x10 * (1)..],
        "freequeue".to_string(),
        g.free_queue(),
    );
    println!();
    print_hex_ptr_global(
        idx * BLOCK_SIZE + 2,
        &buf[0x10 * (2)..],
        "journal".to_string(),
        g.journal(),
    );
    println!();
    print_hex_ptr_global(
        idx * BLOCK_SIZE + 3,
        &buf[0x10 * (3)..],
        "objects".to_string(),
        g.objects(),
    );
    println!();
    print_hex(idx * BLOCK_SIZE + 4, &buf[0x10 * (4)..]);
    print!("directory:{}", g.directory());
    println!();
}
fn print_alloclist(idx: usize, buf: [u8; BLOCK_SIZE]) {
    println!("AllocatorList:");
    let hdr = unsafe { u8_slice_as_any::<LLGHeader>(&buf) };
    print_hex_ptr_global(
        idx * BLOCK_SIZE + 0,
        &buf[0x10 * (0)..],
        "next".to_string(),
        hdr.next,
    );
    println!();
    print_hex(idx * BLOCK_SIZE + 1, &buf[0x10 * 1..]);
    print!("count:{}", hdr.count);
    println!();
    for i in 0..usize::from(hdr.count) {
        let devid = unsafe { u8_slice_as_any::<u64>(&buf[0x20 + i * 24..0x28 + i * 24]) };
        let ptr = unsafe { u8_slice_as_any::<AMPointerGlobal>(&buf[0x28 + i * 24..0x38 + i * 24]) };
        print_hex_ptr_global(
            idx * BLOCK_SIZE + 2 + (i * 3) / 2,
            &buf[0x10 * (2 + (i * 3) / 2)..],
            "alloc".to_string(),
            *ptr,
        );
        print!(" dev:{:x}", devid);
        println!();
        if i % 2 == 0 {
            print_hex(
                idx * BLOCK_SIZE + 2 + (i * 3) / 2 + 1,
                &buf[0x10 * (2 + (i * 3) / 2) + 1..],
            );
            println!();
        }
    }
}
fn print_alloc(idx: usize, buf: [u8; BLOCK_SIZE]) {
    println!("Allocator:");
    let hdr = unsafe { u8_slice_as_any::<LLGHeader>(&buf) };
    print_hex_ptr_global(
        idx * BLOCK_SIZE + 0,
        &buf[0x10 * (0)..],
        "next".to_string(),
        hdr.next,
    );
    println!();
    print_hex(idx * BLOCK_SIZE + 1, &buf[0x10 * 1..]);
    print!("count:{}", hdr.count);
    println!();
    for i in 0..usize::from(hdr.count) {
        if i % 2 == 0 {
            print_hex(idx * BLOCK_SIZE + 2 + (i) / 2, &buf[0x10 * (2 + i / 2)..]);
        }
        let alloc = unsafe { u8_slice_as_any::<u64>(&buf[0x20 + i * 8..0x28 + i * 8]) };
        if i == 0 {
            print!("length:{} ", alloc);
        } else {
            if alloc & 0x8000000000000000 != 0 {
                print!("used:{} ", alloc & 0x7FFFFFFFFFFFFFFF);
            } else {
                print!("free:{} ", alloc);
            }
        }
        if i % 2 == 1 {
            println!();
        }
    }
    if hdr.count % 2 == 1 {
        println!();
    }
}
fn print_free_queue(idx: usize, buf: [u8; BLOCK_SIZE]) {
    println!("Free queue:");
    let hdr = unsafe { u8_slice_as_any::<LLGHeader>(&buf) };
    print_hex_ptr_global(
        idx * BLOCK_SIZE + 0,
        &buf[0x10 * (0)..],
        "next".to_string(),
        hdr.next,
    );
    println!();
    print_hex(idx * BLOCK_SIZE + 1, &buf[0x10 * 1..]);
    print!("count:{}", hdr.count);
    println!();
    for i in 0..usize::from(hdr.count) {
        print_hex(idx * BLOCK_SIZE + 2 + i * 2, &buf[0x10 * (2 + i * 2)..]);
        let txid = unsafe { u8_slice_as_any::<u128>(&buf[0x20 + i * 32..0x30 + i * 32]) };
        println!("txid:{}", txid);
        let ptr = unsafe { u8_slice_as_any::<AMPointerGlobal>(&buf[0x30 + i * 32..0x40 + i * 32]) };
        print_hex_ptr_global(
            idx * BLOCK_SIZE + 2 + i * 2,
            &buf[0x10 * (2 + i * 2)..],
            "block".to_string(),
            *ptr,
        );
        println!();
    }
    println!();
}
fn print_objs(idx: usize, buf: [u8; BLOCK_SIZE], _o: ObjectSet, _d: &DiskGroup) {
    println!("ObjectSet:");
    let hdr = unsafe { u8_slice_as_any::<ObjectListHeader>(&buf) };
    print_hex(idx * BLOCK_SIZE, &buf[0..]);
    print!("start:{} count:{}", hdr.start_idx, hdr.n_entries);
    println!();
    let mut pos = std::mem::size_of::<ObjectListHeader>();
    for _ in 0..usize::try_from(hdr.n_entries).unwrap() {
        loop {
            let blkof = pos / 16;
            let size = u64::from_le_bytes(buf[pos..pos + 8].try_into().unwrap());
            print_hex(idx * BLOCK_SIZE + blkof, &buf[blkof * 16..blkof * 16 + 16]);
            print!("size:{} ", size);
            if size == 0 {
                pos += 8;
                println!();
                break;
            }
            let offset = u64::from_le_bytes(buf[pos + 8..pos + 16].try_into().unwrap());
            print!("offs:{} ", offset);
            println!();
            let ptr = unsafe { u8_slice_as_any::<AMPointerGlobal>(&buf[pos + 16..pos + 32]) };
            print_hex_ptr_global(
                idx * BLOCK_SIZE + blkof + 1,
                &buf[blkof * 16 + 16..blkof * 16 + 32],
                "data".to_string(),
                *ptr,
            );
            println!();
            pos += std::mem::size_of::<Fragment>();
        }
    }
}
fn print_geometry(idx: usize, buf: [u8; BLOCK_SIZE], g: Geometry, _d: &Disk) {
    println!("Geometry:");
    for i in 0..255 {
        if buf[0x10 * i..0x10 * (i + 1)] == [0; 16] {
            continue;
        }
        print_hex(idx * BLOCK_SIZE + i, &buf[0x10 * i..]);
        if g.device_ids[i * 2] != 0 {
            print!("dev{}:{:08x}", i * 2, { g.device_ids[i * 2] });
        }
        if g.device_ids[i * 2 + 1] != 0 {
            print!("dev{}:{:08x}", i * 2 + 1, { g.device_ids[i * 2 + 1] });
        }
        println!();
    }
    print_hex(idx * BLOCK_SIZE + 255, &buf[0x10 * 255..]);
    print!("{:?}", g.flavor);
    println!();
}
fn print_superblock(idx: usize, buf: [u8; BLOCK_SIZE], mut s: Superblock, d: &Disk) {
    println!("Superblock:");
    print_hex(idx * BLOCK_SIZE + 0, &buf[0x00..]);
    if buf[0..8] == *SIGNATURE {
        print!("sig:{:8} ", String::from_utf8_lossy(s.signature()).green())
    } else {
        print!("sig:{:8} ", String::from_utf8_lossy(s.signature()).red())
    }
    print!("dev:{:016x} ", s.devid());
    println!();

    let features: HashMap<usize, AMFeatures> =
        AMFeatures::iter().map(|f| (f as usize, f)).collect();

    for i in 0..16 {
        if (i * 128..(i + 1) * 128).all(|x| !features.contains_key(&x)) {
            continue;
        }
        print_hex(idx * BLOCK_SIZE + 1 + i, &buf[0x10 * (1 + i)..]);
        for j in 0..16 {
            for k in 0..8 {
                let f: usize = i * 128 + j * 8 + k;
                if !features.contains_key(&(f)) {
                    continue;
                } else {
                    if *s.features().get(f).unwrap() {
                        print!("{} ", format!("{:?}", features[&f]).green());
                    } else {
                        print!("{} ", format!("{:?}", features[&f]).red());
                    }
                }
            }
        }
        println!();
    }

    for i in 0..16 {
        if s.geometries(i).is_null() {
            continue;
        }
        print_hex_ptr_local(
            idx * BLOCK_SIZE + 17 + i,
            &buf[0x10 * (17 + i)..],
            format!("geom{}", i),
            s.geometries(i),
            d,
        );
        println!();
    }

    if s.verify_checksum() {
        print!("\t{:06x} : ", (idx * BLOCK_SIZE + 33) * 0x10);
        for i in 0..4 {
            print!("{}", format!("{:02x} ", buf[0x10 * 33 + i]).green());
        }
        for i in 4..16 {
            print!("{:02x} ", buf[0x10 * 33 + i]);
        }
        print!("| ");
        print!("sum:{} ", format!("{:8x}", s.checksum()).green())
    } else {
        print!("\t{:06x} : ", (idx * BLOCK_SIZE + 33) * 0x10);
        for i in 0..4 {
            print!("{}", format!("{:02x} ", buf[0x10 * 33 + i]).red());
        }
        for i in 4..16 {
            print!("{:02x} ", buf[0x10 * 33 + i]);
        }
        print!("| ");
        print!("sum:{} ", format!("{:8x}", s.checksum()).red())
    }
    println!();

    print_hex(idx * BLOCK_SIZE + 127, &buf[0x10 * 127..]);
    print!("latest:{} ", s.latest_root());
    println!();

    for i in 0..128 {
        if s.rootnodes(i).is_null() {
            continue;
        }
        print_hex_ptr_global(
            idx * BLOCK_SIZE + 128 + i,
            &buf[0x10 * (128 + i)..],
            format!("root{}", i),
            s.rootnodes(i),
        );
        println!();
    }
}
fn print_error(_idx: usize, _buf: [u8; BLOCK_SIZE]) {
    unimplemented!();
}
fn print_hex(idx: usize, data: &[u8]) {
    print!("\t{:06x} : ", idx * 0x10);
    for i in 0..16 {
        print!("{:02x} ", data[i]);
    }
    print!("| ");
}
fn print_hex_ptr_local(idx: usize, data: &[u8], name: String, p: AMPointerLocal, d: &Disk) {
    print!("\t{:06x} : ", idx * 0x10);
    for i in 0..8 {
        print!("{:02x} ", data[i]);
    }
    for i in 8..12 {
        if p.validate(d.clone()).unwrap() {
            print!("{}", format!("{:02x} ", data[i]).green());
        } else {
            print!("{}", format!("{:02x} ", data[i]).red());
        }
    }
    for i in 12..16 {
        print!("{:02x} ", data[i]);
    }
    print!("| ");
    print!("{}:{:08x}", name, p.loc());
}
fn print_hex_ptr_global(idx: usize, data: &[u8], name: String, p: AMPointerGlobal) {
    print!("\t{:06x} : ", idx * 0x10);
    for i in 0..16 {
        print!("{:02x} ", data[i]);
    }
    print!("| ");
    if p.is_null() {
        print!("{}:NULL", name);
    } else {
        print!("{}:{},{},{:08x}", name, p.geo(), p.dev(), p.loc());
    }
}
