// An `enumerate()` made in place of what isn't an array's items, a `fuse()`
// of owned items with a destructor, stepped from its back: `$enumerate` of
// its source made knowing its back and its length, each index where it is,
// each item dropped as Rust drops it (ADR 0528; #383).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn louds(n: u32) -> Vec<Loud> {
    (0..n).map(Loud).collect()
}

fn main() {
    let e = louds(4)
        .into_iter()
        .fuse()
        .enumerate()
        .rev()
        .nth(1)
        .map(|(i, l)| (i, l.0));
    println!("{e:?}");
    let back: Vec<(usize, u32)> = louds(3)
        .into_iter()
        .fuse()
        .enumerate()
        .rev()
        .map(|(i, l)| (i, l.0))
        .collect();
    println!("{back:?}");
    let ranged: Vec<(usize, u32)> = (5..9u32).fuse().enumerate().rev().take(2).collect();
    println!("{ranged:?}");
}
