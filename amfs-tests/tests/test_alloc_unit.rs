use amfs::Allocator;

#[test]
fn basic() {
    let a = Allocator::new(1024);
    assert_eq!(a.used_space(), 0);
    assert_eq!(a.free_space(), 1024);
    assert_eq!(a.total_space(), 1024);
}

#[test]
fn alloc() {
    let mut a = Allocator::new(1024);
    let blk = a.alloc_blocks(2);
    assert!(blk.ok() != None);
    assert_eq!(a.used_space(), 2);
    assert_eq!(a.free_space(), 1022);
}

#[test]
fn free() {
    let mut a = Allocator::new(1024);
    let blk = a.alloc_blocks(2);
    a.free(blk.unwrap()).unwrap();
    assert_eq!(a.used_space(), 0);
    assert_eq!(a.free_space(), 1024);
}

#[test]
fn alloc_fill() {
    let mut a = Allocator::new(1024);
    let blk = a.alloc_blocks(512);
    assert!(blk.ok() != None);
    let blk = a.alloc_blocks(512);
    assert!(blk.ok() != None);
    let blk = a.alloc_blocks(2);
    assert!(blk.ok() == None);
}

#[test]
fn alloc_frag() {
    let mut a = Allocator::new(1024);
    let b1 = a.alloc_blocks(512);
    let b2 = a.alloc_blocks(512);
    a.free(b1.unwrap()).unwrap();
    a.free(b2.unwrap()).unwrap();
    let blk = a.alloc_blocks(1024);
    assert!(blk.ok() != None);
}

#[test]
fn mark_used() {
    let mut a = Allocator::new(1024);
    a.mark_used(0, 1).unwrap();
    a.mark_used(1, 1).unwrap();
    a.mark_used(1023, 1).unwrap();
    a.mark_used(1022, 1).unwrap();
    a.mark_used(510, 2).unwrap();
    a.mark_used(512, 2).unwrap();
    a.mark_used(2, 508).unwrap();
    a.mark_used(514, 508).unwrap();
    let blk = a.alloc_blocks(1);
    assert!(blk.ok() == None);
    a.free(0).unwrap();
    a.free(1).unwrap();
    a.free(2).unwrap();
    a.free(510).unwrap();
    a.free(512).unwrap();
    a.free(514).unwrap();
    a.free(1022).unwrap();
    a.free(1023).unwrap();
    let blk = a.alloc_blocks(1024);
    assert!(blk.ok() != None);
}

#[test]
fn alloc_many() {
    let mut a = Allocator::new(1024);
    a.mark_used(0, 1).unwrap();
    a.mark_used(1, 1).unwrap();
    a.mark_used(510, 2).unwrap();
    a.mark_used(512, 2).unwrap();
    a.mark_used(1023, 1).unwrap();
    a.mark_used(1022, 1).unwrap();
    let blk = a.alloc_many(1016).unwrap();
    assert!(blk.len() == 1016);
    for b in blk {
        a.free(b).unwrap();
    }
    let blk = a.alloc_many(1017);
    assert!(blk.ok() == None);
}

#[test]
fn alloc_many_free_on_fail() {
    let mut a = Allocator::new(1024);
    let blk = a.alloc_many(1025);
    assert!(blk.ok() == None);
    let blk = a.alloc_blocks(1024);
    assert!(blk.ok() != None);
}
