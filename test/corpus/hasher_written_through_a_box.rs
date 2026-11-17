//@ library-differs
// A std hasher in a `Box`, written through it: std's `impl Hasher for
// Box<H>` writes each of `Hasher`'s methods as `H`'s, so each is the
// hasher's own, and its `finish` is what the same writes give a hasher
// held as it is (#1493). Each method std forwards that stable Rust can
// call; through a `&mut` to it and a `&mut` to that; a value's `hash`
// into it; the box given to generic code as an `H: Hasher`, and through a
// `&mut` to it; and a `Box<dyn Hasher>`.
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn write_each<H: Hasher>(h: &mut H) {
    h.write(&[1, 2, 3]);
    h.write_u8(2);
    h.write_u16(3);
    h.write_u32(4);
    h.write_u64(5);
    h.write_u128(6);
    h.write_usize(7);
    h.write_i8(-8);
    h.write_i16(-9);
    h.write_i32(-10);
    h.write_i64(-11);
    h.write_i128(-12);
    h.write_isize(-13);
}

fn fed<H: Hasher>(mut h: H) -> u64 {
    write_each(&mut h);
    h.finish()
}

fn main() {
    let mut plain = DefaultHasher::new();
    write_each(&mut plain);
    let expected = plain.finish();

    let mut b: Box<DefaultHasher> = Box::new(DefaultHasher::new());
    b.write(&[1, 2, 3]);
    b.write_u8(2);
    b.write_u16(3);
    b.write_u32(4);
    b.write_u64(5);
    b.write_u128(6);
    b.write_usize(7);
    b.write_i8(-8);
    b.write_i16(-9);
    b.write_i32(-10);
    b.write_i64(-11);
    b.write_i128(-12);
    b.write_isize(-13);
    println!("each method {}", b.finish() == expected);

    let r = &mut b;
    r.write_u8(1);
    let mut again = DefaultHasher::new();
    write_each(&mut again);
    again.write_u8(1);
    println!("through a &mut {}", b.finish() == again.finish());

    let mut twice: Box<DefaultHasher> = Box::new(DefaultHasher::new());
    let rr = &mut &mut twice;
    rr.write_u8(1);
    let mut one = DefaultHasher::new();
    one.write_u8(1);
    println!("through a &mut &mut {}", twice.finish() == one.finish());

    println!("generic {}", fed(Box::new(DefaultHasher::new())) == expected);
    let mut given: Box<DefaultHasher> = Box::new(DefaultHasher::new());
    write_each(&mut given);
    println!("generic &mut {}", given.finish() == expected);

    let mut hashed: Box<DefaultHasher> = Box::new(DefaultHasher::new());
    (7u32, "seven").hash(&mut hashed);
    let mut plain_hashed = DefaultHasher::new();
    (7u32, "seven").hash(&mut plain_hashed);
    println!("hash into it {}", hashed.finish() == plain_hashed.finish());

    let mut d: Box<dyn Hasher> = Box::new(DefaultHasher::new());
    write_each(&mut d);
    println!("dyn {}", d.finish() == expected);
}
