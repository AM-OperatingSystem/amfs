use std::fs::File;

use amfs::{
    AMPointerGlobal, AMPointerLocal, DiskGroup, FSGroup, Geometry, Superblock, BLOCK_SIZE,
    SIGNATURE,
};

/// Zero-filled file
pub fn generate_0000(f: &File) {
    super::utils::create_file(f, 1000)
}

/// Create valid superblock signature
pub fn generate_0001(f: &File) {
    generate_0000(f);

    let mut d = super::utils::get_disk(f);
    let locs = d.get_header_locs().unwrap();
    for i in locs {
        let mut res = [0u8; BLOCK_SIZE];
        d.read_at(i.loc(), &mut res).unwrap();
        res[..8].clone_from_slice(SIGNATURE);
        d.write_at(i.loc(), &res).unwrap();
    }
}

/// Create valid superblock checksum
pub fn generate_0002(f: &File) {
    generate_0001(f);

    let mut d = super::utils::get_disk(f);
    let locs = d.get_header_locs().unwrap();
    for i in locs {
        let mut sb: Superblock = Superblock::new(0);
        d.read_at(i.loc(), &mut sb).unwrap();
        sb.update_checksum();
        d.write_at(i.loc(), &sb).unwrap();
    }
}

/// Populate diskids
pub fn generate_0003(f: &File) {
    generate_0002(f);

    let mut d = super::utils::get_disk(f);
    let locs = d.get_header_locs().unwrap();
    for i in locs {
        let mut res = [0u8; BLOCK_SIZE];
        d.read_at(i.loc(), &mut res).unwrap();
        res[8..16].clone_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        d.write_at(i.loc(), &res).unwrap();

        let mut sb: Superblock = Superblock::new(0);
        d.read_at(i.loc(), &mut sb).unwrap();
        sb.update_checksum();
        d.write_at(i.loc(), &sb).unwrap();
    }
}

/// Build geometry, incorrect diskid
pub fn generate_0004(f: &File) {
    generate_0003(f);

    let mut d = super::utils::get_disk(f);
    let locs = d.get_header_locs().unwrap();
    for i in locs {
        let mut geo = Geometry::new();
        geo.device_ids[0] = 0x1;
        d.write_at(2, &geo).unwrap();

        let mut ptr = AMPointerLocal::new(2);
        ptr.update(d.clone()).unwrap();

        let mut res = [0u8; BLOCK_SIZE];
        d.read_at(i.loc(), &mut res).unwrap();
        res[272..288].clone_from_slice(&ptr.as_bytes());
        d.write_at(i.loc(), &res).unwrap();

        let mut sb: Superblock = Superblock::new(0);
        d.read_at(i.loc(), &mut sb).unwrap();
        sb.update_checksum();
        d.write_at(i.loc(), &sb).unwrap();
    }
}

/// Build geometry, correct diskid
pub fn generate_0005(f: &File) {
    generate_0003(f);

    let mut d = super::utils::get_disk(f);
    let locs = d.get_header_locs().unwrap();
    for i in locs {
        let mut geo = Geometry::new();
        geo.device_ids[0] = 0x0807060504030201;
        d.write_at(2, &geo).unwrap();

        let mut ptr = AMPointerLocal::new(2);
        ptr.update(d.clone()).unwrap();

        let mut res = [0u8; BLOCK_SIZE];
        d.read_at(i.loc(), &mut res).unwrap();
        res[272..288].clone_from_slice(&ptr.as_bytes());
        d.write_at(i.loc(), &res).unwrap();

        let mut sb: Superblock = Superblock::new(0);
        d.read_at(i.loc(), &mut sb).unwrap();
        sb.update_checksum();
        d.write_at(i.loc(), &sb).unwrap();
    }
}

/// Build diskgroup
pub fn generate_0006(f: &File) {
    generate_0005(f);

    let mut d = super::utils::get_disk(f);
    let locs = d.get_header_locs().unwrap();

    let mut geo = Geometry::new();
    geo.device_ids[0] = 0x0807060504030201;
    let dg = DiskGroup::from_geo(geo, &[0x0807060504030201], &[d.clone()]).unwrap();

    for i in locs {
        let fsg = FSGroup::new();
        d.write_at(3, &fsg).unwrap();

        let mut ptr = AMPointerGlobal::new(3, 1, 0, 0);
        ptr.update(&[Some(dg.clone())]).unwrap();

        let mut res = [0u8; BLOCK_SIZE];
        d.read_at(i.loc(), &mut res).unwrap();
        res[2048..2064].clone_from_slice(&ptr.as_bytes());
        d.write_at(i.loc(), &res).unwrap();

        let mut sb: Superblock = Superblock::new(0);
        d.read_at(i.loc(), &mut sb).unwrap();
        sb.update_checksum();
        d.write_at(i.loc(), &sb).unwrap();
    }
}
