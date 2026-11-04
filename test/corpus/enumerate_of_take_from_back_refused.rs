//@ compile-fail: as an `ExactSizeIterator`, which rust-js can't count
// An `enumerate()` of a `take()` of items with a destructor, folded from
// its back: its index from the back is its source's length, which a
// `take()` of them can't count, so it's refused where it's made, never
// stepped by a back and a length its JS lacks (#651).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let ds: Vec<D> = (1..7).map(D).collect();
    let n = ds.into_iter().take(4).enumerate().rfold(0, |n, (i, d)| n * 10 + i as u32 + d.0);
    println!("{n}");
}
