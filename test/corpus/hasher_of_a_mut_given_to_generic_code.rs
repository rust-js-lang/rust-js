// The crate's own `impl Hasher for &mut Tally`, whose `finish` counts, no
// forwarding, given to generic code as its `H`: a `&mut Tally` it holds in
// its box, whose dictionary is the impl's, never std's `impl Hasher for
// &mut H` asking a `Tally: Hasher` there's none of (#1730). Generic code's
// own `write`, std's `write_u8` default over it, `Hash` of a number and of
// text, and `finish`, each the impl's; and a `write` that points the `&mut`
// at another `Tally`, which the caller's `t` is then, as the `&mut` it held
// is written back.
use std::hash::{Hash, Hasher};

struct Tally {
    bytes: u64,
}

impl Hasher for &mut Tally {
    fn finish(&self) -> u64 {
        self.bytes * 1000
    }

    fn write(&mut self, bytes: &[u8]) {
        if bytes.first() == Some(&b't') {
            *self = Box::leak(Box::new(Tally { bytes: 100 }));
        }
        self.bytes += bytes.len() as u64;
    }
}

fn feed<H: Hasher>(h: &mut H) -> u64 {
    h.write(b"xy");
    h.write_u8(1);
    h.finish()
}

fn fed<T: Hash, H: Hasher>(value: &T, h: &mut H) -> u64 {
    value.hash(h);
    h.finish()
}

fn main() {
    let mut tally = Tally { bytes: 0 };
    let mut t = &mut tally;
    println!("{}", feed(&mut t));
    println!("{}", fed(&7u32, &mut t));
    "text".hash(&mut t);
    println!("{}", t.finish());
    feed(&mut t);
    println!("{} {}", t.finish(), tally.bytes);
}
