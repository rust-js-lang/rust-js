// `finish()` of a `&mut &mut` to the crate's own hasher, which Rust finds
// in std's `impl Hasher for &mut H` twice over, as it takes `&self`: each
// only forwards, so it's the hasher's own, through the `&mut` the outer one
// is to (#1724). And a `&mut` to a type whose own `impl Hasher for &mut
// Tally` has a body: std's impl passes the call on to that impl, never past
// it to the type, which has no `Hasher`. (Given to generic code, it's
// `hasher_of_a_mut_given_to_generic_code`'s.)
use std::hash::Hasher;

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

struct Tally {
    bytes: u64,
}

impl Hasher for &mut Tally {
    fn finish(&self) -> u64 {
        self.bytes * 1000
    }

    fn write(&mut self, bytes: &[u8]) {
        self.bytes += bytes.len() as u64;
    }
}

fn twice(rr: &mut &mut Fnv) -> u64 {
    rr.write(&[2, 3]);
    rr.finish()
}

fn main() {
    let mut h = Fnv(0xcbf2_9ce4_8422_2325);
    let mut r = &mut h;
    let rr = &mut r;
    rr.write_u8(1);
    println!("{}", rr.finish());
    println!("{}", twice(&mut &mut h));
    println!("{}", h.finish());

    let mut tally = Tally { bytes: 0 };
    let mut t = &mut tally;
    t.write(b"text");
    println!("{}", t.finish());
    let tt = &mut t;
    tt.write(&[1, 2, 3]);
    println!("{}", tt.finish());
    println!("{}", tally.bytes);
}
