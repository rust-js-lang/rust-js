// `finish()` of a `&mut` to std's hasher, which Rust finds in std's `impl
// Hasher for &mut H`, as it takes `&self`: the hasher's own, as that impl
// forwards to it, through a function's `&mut`, a `let`'s, one whose hasher
// is replaced, a field's, `iter_mut()`'s, `as_mut()`'s and a `RefMut`'s. It
// threw, given the hasher where std's impl's dictionary read a box (#1488).
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn feed(h: &mut DefaultHasher) -> u64 {
    h.write_u8(1);
    h.finish()
}

struct Holder {
    h: DefaultHasher,
}

fn main() {
    let mut h = DefaultHasher::new();
    println!("{}", feed(&mut h));
    let r = &mut h;
    println!("{}", r.finish());
    r.write_u16(6);
    7u8.hash(r);
    let f = r.finish();
    println!("{f} {}", (&h).finish());
    let mut g = DefaultHasher::new();
    let r = &mut g;
    r.write_u8(3);
    *r = DefaultHasher::new();
    r.write_u8(4);
    println!("{}", r.finish());
    let mut holder = Holder { h: DefaultHasher::new() };
    let hr = &mut holder.h;
    hr.write_u8(5);
    println!("{} {}", hr.finish(), holder.h.finish());
    let mut hs = [DefaultHasher::new(), DefaultHasher::new()];
    for h in hs.iter_mut() {
        h.write_u8(2);
        println!("{}", h.finish());
    }
    let mut o = Some(DefaultHasher::new());
    if let Some(h) = o.as_mut() {
        h.write_u8(8);
        println!("{}", h.finish());
    }
    let mut c = std::cell::RefCell::new(DefaultHasher::new());
    let mut b = c.borrow_mut();
    b.write_u8(9);
    let m: &mut DefaultHasher = &mut b;
    println!("{}", m.finish());
    drop(b);
    println!("{}", c.get_mut().finish());
}
