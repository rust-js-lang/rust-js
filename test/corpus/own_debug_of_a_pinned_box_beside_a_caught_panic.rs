// The crate's own `Debug` of std's type, `impl Debug for Pin<Box<Pinned>>`,
// in a crate that catches a panic: it writes its text as it does, no std
// value's parts that could stop part-way, so `{:?}` of it is its call, not
// refused as a composed one (#1894).
use std::fmt;
use std::pin::Pin;

struct Pinned(u8);

impl fmt::Debug for Pin<Box<Pinned>> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P{}", self.0)
    }
}

fn main() {
    let p = Box::pin(Pinned(6));
    println!("{:?} {}", p, format!("{p:?}"));
    let caught = std::panic::catch_unwind(|| {
        if p.0 > 5 {
            panic!("big");
        }
    });
    println!("{}", caught.is_err());
}
