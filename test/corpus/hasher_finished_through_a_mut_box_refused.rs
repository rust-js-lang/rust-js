//@ compile-fail: `finish` of a `&mut` to a `Box` of a hasher (#2037)
// `finish()` of a `&mut` to a `Box` of std's hasher, a local's or a
// parameter's: std's `impl Hasher for &mut H` of `H = Box<DefaultHasher>`,
// of `&self`, which the `&mut`'s pass-through takes only where the `&mut`
// is the box itself, not a cell of it (#2037): refused, naming the issue.
use std::collections::hash_map::DefaultHasher;
use std::hash::Hasher;

fn finished(h: &mut Box<DefaultHasher>) -> u64 {
    h.write_u8(1);
    h.finish()
}

fn main() {
    let mut b: Box<DefaultHasher> = Box::new(DefaultHasher::new());
    let r = &mut b;
    r.write_u8(1);
    println!("{}", r.finish() != 0);
    println!("{}", finished(&mut Box::new(DefaultHasher::new())) != 0);
}
