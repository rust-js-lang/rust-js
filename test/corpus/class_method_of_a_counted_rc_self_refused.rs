//@ compile-fail: a `#[rust_js::class]` method whose `self` is
// A class's method whose `self` is an `Rc` of it the crate counts: JS calls
// it on the instance, Rust's callers have the box that counts it, and no
// one method takes both (#1227, the user's decision): refused, as a
// constructor that gives one is.
use std::rc::Rc;

#[cfg_attr(rust_js, rust_js::class)]
struct Chunk {
    n: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Chunk {
    fn new(n: u32) -> Self {
        Chunk { n }
    }
    fn watch(self: &Rc<Self>) -> u32 {
        self.n + Rc::strong_count(self) as u32
    }
}

fn main() {
    let chunk = Rc::new(Chunk::new(5));
    let other = chunk.clone();
    println!("{} {}", chunk.watch(), other.watch());
}
