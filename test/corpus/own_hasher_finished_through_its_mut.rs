// `finish()` of a `&mut` to the crate's own hasher, which Rust finds in
// std's `impl Hasher for &mut H`, as it takes `&self`: the hasher's own, as
// that impl forwards to it. Its dictionary read the hasher through a box it
// isn't, a `TypeError` (#1709). And `hash` given the `&mut`, generic code's.
use std::hash::{Hash, Hasher};

struct Fnv(u64);

impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = (self.0 ^ u64::from(*byte)).wrapping_mul(0x100_0000_01b3);
        }
    }
}

fn finished(h: &mut Fnv) -> u64 {
    h.write_u8(1);
    h.finish()
}

fn main() {
    let mut h = Fnv(0xcbf2_9ce4_8422_2325);
    println!("{}", finished(&mut h));
    let r = &mut h;
    "text".hash(r);
    println!("{}", r.finish());
    r.write(&[2, 3]);
    println!("{} {}", r.finish(), h.finish());
}
