// A `take(n)` advanced from its back trims its source to the `n` it gives
// and advances it as far again in one `advance_back_by` of its source's,
// as std's `Take::advance_back_by` does: a `Vec`'s drops all of what it
// passes as one slice, front to back, not the trimmed ones first (#2121).

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    (1..=5).map(D).collect()
}

fn main() {
    let n = ds().into_iter().take(3).rev().take(2).count();
    println!("take rev take count {n}");
    let left: Vec<u32> = ds().into_iter().take(4).rev().skip(1).map(|d| d.0).collect();
    println!("take rev skip {left:?}");
    let n = ds().into_iter().take(9).rev().take(2).count();
    println!("past its source {n}");
}
