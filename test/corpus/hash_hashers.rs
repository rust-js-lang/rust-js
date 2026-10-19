//@ native: wasm32
// Hashers (ADR 0463): a crate's own, given each write std's `Hash` impls
// make, its provided methods std's over what it writes itself; generic
// code's `H: Hasher`; a `dyn Hasher`; and std's deprecated `SipHasher`,
// SipHash-2-4, of keys.

#[allow(deprecated)]
use std::hash::SipHasher;
use std::hash::{DefaultHasher, Hash, Hasher};

// What it's given, a write at a time: `write_u64` its own, so std's
// `write_i64` writes through it.
#[derive(Default)]
struct Tally(Vec<String>);

impl Hasher for Tally {
    fn finish(&self) -> u64 {
        self.0.len() as u64
    }

    fn write(&mut self, bytes: &[u8]) {
        self.0.push(format!("{bytes:?}"));
    }

    fn write_u64(&mut self, n: u64) {
        self.0.push(format!("u64 {n}"));
    }
}

// FNV-1a, as hashers written by hand are.
struct Fnv(u64);

impl Default for Fnv {
    fn default() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
        }
    }
}

#[derive(Hash)]
struct Entry {
    key: String,
    rank: Option<u32>,
}

fn written<T: Hash>(value: &T) -> Vec<String> {
    let mut tally = Tally::default();
    value.hash(&mut tally);
    tally.0
}

fn fnv<T: Hash>(value: &T) -> u64 {
    let mut hasher = Fnv::default();
    value.hash(&mut hasher);
    hasher.finish()
}

fn write_twice<H: Hasher>(hasher: &mut H, n: u32) {
    hasher.write_u32(n);
    n.hash(hasher);
}

fn through_dyn(hasher: &mut dyn Hasher, n: u16) {
    hasher.write_u16(n);
    n.hash(&mut &mut *hasher);
}

fn main() {
    println!("{:?}", written(&(-3i64, 4u64, 'x')));
    println!("{:?}", written(&vec![Some(1u8), None]));
    println!("{:?}", written(&"ab"));
    println!("{:?}", written(&[1u16, 2]));
    println!("{:?}", written(&Entry { key: "k".into(), rank: Some(2) }));
    println!("{} {}", fnv(&"hello"), fnv(&Entry { key: "k".into(), rank: None }));

    let mut tally = Tally::default();
    write_twice(&mut tally, 1);
    tally.write_i64(-1);
    tally.write_usize(5);
    println!("{:?} {}", tally.0, tally.finish());

    let mut hasher = DefaultHasher::new();
    write_twice(&mut hasher, 7);
    through_dyn(&mut hasher, 9);
    println!("{}", hasher.finish());
    let mut fnv = Fnv::default();
    through_dyn(&mut fnv, 9);
    println!("{}", fnv.finish());

    #[allow(deprecated)]
    let (mut keyed, mut plain) = (SipHasher::new_with_keys(3, 4), SipHasher::new());
    Entry { key: "k".into(), rank: Some(1) }.hash(&mut keyed);
    plain.write(b"abcdefghij");
    println!("{} {}", keyed.finish(), plain.finish());
}
