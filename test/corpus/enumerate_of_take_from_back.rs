// An `enumerate()` of a `take()` of items with a destructor, folded from
// its back: its index from the back is its source's length, which a
// `take()` of them, `$taking(source, n, drop)`, counts, made knowing its
// back and its length where it's made (#718). Refused once, as `.take(n)`
// knew neither and would have thrown (#651). What it trims, `D(6)` and
// `D(5)`, is dropped first, as std's `Take::rfold` drops what its source's
// `nth_back` passes and finds.
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
