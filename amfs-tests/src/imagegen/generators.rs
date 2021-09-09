use std::fs::File;

/// Zero-filled file
pub fn generate_0000(f: &File) {
    super::utils::create_file(f,1000)
}

/// Create valid superblock signature
pub fn generate_0001(f: &File) {
    use amfs::BLOCK_SIZE;
    use amfs::SIGNATURE;

    generate_0000(f);

    let mut d = super::utils::get_disk(f);
    let locs = d.get_header_locs().unwrap();
    for i in locs {
        let mut res = [0u8;BLOCK_SIZE];
        d.read_at(i.loc(), &mut res).unwrap();
        res[..8].clone_from_slice(SIGNATURE);
        d.write_at(i.loc(), &res).unwrap();
    }
}

/// Create valid superblock checksum
pub fn generate_0002(f: &File) {
    use amfs::Superblock;

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

/// Create valid superblock checksum
pub fn generate_0003(f: &File) {
    use amfs::BLOCK_SIZE;
    use amfs::Superblock;

    generate_0002(f);

    let mut d = super::utils::get_disk(f);
    let locs = d.get_header_locs().unwrap();
    for i in locs {
        let mut res = [0u8;BLOCK_SIZE];
        d.read_at(i.loc(), &mut res).unwrap();
        res[8..16].clone_from_slice(&[1,2,3,4,5,6,7,8]);
        d.write_at(i.loc(), &res).unwrap();

        let mut sb: Superblock = Superblock::new(0);
        d.read_at(i.loc(), &mut sb).unwrap();
        sb.update_checksum();
        d.write_at(i.loc(), &sb).unwrap();
    }
}