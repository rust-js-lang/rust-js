//@ native: wasm32
// A hash set that holds items reserves half the lower bound it's asked,
// rounded up, as hashbrown's `extend_reserve` does: room for 150 million
// `u32`s fits a 32-bit target, where 300 million would panic. Native Rust
// here is `wasm32-wasip1`'s, whose `usize` is rust-js's (ADR 0090; #2414).
// An iterator may hint more than it gives.
use std::collections::HashSet;

struct Claims(u32);

impl Iterator for Claims {
    type Item = (u32, u32);
    fn next(&mut self) -> Option<(u32, u32)> {
        self.0 += 1;
        (self.0 <= 2).then_some((self.0, self.0))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (300_000_000, None)
    }
}

fn main() {
    println!("usize {}", usize::MAX);
    let mut pair: (HashSet<u32>, Vec<u32>) = (HashSet::from([7]), Vec::new());
    pair.extend(Claims(0));
    println!("holding: {}", pair.0.len());
}
