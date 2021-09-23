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
    let blk = a.alloc(2);
    assert!(blk != None);
    assert_eq!(a.used_space(), 2);
    assert_eq!(a.free_space(), 1022);
}

#[test]
fn free() {
    let mut a = Allocator::new(1024);
    let blk = a.alloc(2);
    a.free(blk.unwrap());
    assert_eq!(a.used_space(), 0);
    assert_eq!(a.free_space(), 1024);
}

#[test]
fn alloc_fill() {
    let mut a = Allocator::new(1024);
    let blk = a.alloc(512);
    assert!(blk != None);
    let blk = a.alloc(512);
    assert!(blk != None);
    let blk = a.alloc(2);
    assert!(blk == None);
}

#[test]
fn alloc_frag() {
    let mut a = Allocator::new(1024);
    let b1 = a.alloc(512);
    let b2 = a.alloc(512);
    a.free(b1.unwrap());
    a.free(b2.unwrap());
    let blk = a.alloc(1024);
    assert!(blk != None);
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
    let blk = a.alloc(1);
    assert!(blk == None);
    a.free(0);
    a.free(1);
    a.free(2);
    a.free(510);
    a.free(512);
    a.free(514);
    a.free(1022);
    a.free(1023);
    let blk = a.alloc(1024);
    assert!(blk != None);
}
