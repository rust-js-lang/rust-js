//@ native: wasm32
// Generic code bounded by `Hasher`, given std's hasher: by value, through
// `&mut`, `&` and a `Box`, `&mut &mut` and a `Default` one made there, each write a `&mut self` of its
// cell and `finish` a `&self` of the hasher itself, as std's dictionary
// takes them. A `DefaultHasher` given by value threw at its `finish()`,
// which read the hasher's cell where generic code gave the hasher (#1477).
// `usize` as wasm32's.
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher, Hash, Hasher};

fn by_value<H: Hasher>(mut h: H) -> u64 {
    h.write(&[1, 2, 3]);
    h.write_u8(4);
    h.write_u32(5);
    h.write_u64(6);
    h.write_i64(-7);
    h.write_usize(8);
    h.finish()
}

fn by_ref<H: Hasher>(h: &mut H) -> u64 {
    h.write_u8(9);
    h.finish()
}

fn finished<H: Hasher>(h: &H) -> u64 {
    h.finish()
}

fn hashed<T: Hash, H: Hasher>(t: &T, h: &mut H) -> u64 {
    t.hash(h);
    h.finish()
}

fn owned<H: Hasher + Default>() -> u64 {
    let mut h = H::default();
    h.write_u8(1);
    h.finish()
}

fn main() {
    println!("{}", by_value(DefaultHasher::new()));
    let mut h = DefaultHasher::new();
    println!("{}", by_value(&mut h));
    println!("{} {}", by_ref(&mut h), h.finish());
    println!("{}", finished(&h));
    let mut boxed: Box<DefaultHasher> = Box::new(DefaultHasher::new());
    println!("{} {}", by_ref(&mut boxed), by_value(boxed));
    let mut g = DefaultHasher::new();
    println!("{} {}", hashed(&"x", &mut g), hashed(&(1u8, 'c'), &mut &mut g));
    let built = BuildHasherDefault::<DefaultHasher>::default().build_hasher();
    println!("{} {}", by_value(built), owned::<DefaultHasher>());
}
