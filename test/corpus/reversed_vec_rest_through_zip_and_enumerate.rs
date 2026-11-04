// What a reversed `Vec`'s `into_iter()` still holds, through a `zip` and a
// `step_by`, or an `enumerate` and a `take`, is dropped as std drops it
// (#654). A `step_by()` steps its source by `next` and `nth`, so a zip
// whose second side ends takes an item more of its first and drops it,
// where its `TrustedLen` fold would stop at the shorter side; then the
// `Vec`'s rest is dropped as one slice, front to back. A `take(n).count()`
// is its source's `advance_by(n)`, an `enumerate`'s its source's, so a
// reversed `Vec`'s drops what it passes as one slice, front to back too.
use std::collections::HashSet;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5)]
}

fn main() {
    let total = ds()
        .into_iter()
        .rev()
        .zip(10..13u32)
        .step_by(2)
        .fold(0, |a, (d, n)| a + d.0 * 100 + n);
    println!("fold {total}");
    let n = ds().into_iter().rev().zip(10..13u32).step_by(2).count();
    println!("count {n}");
    let last = ds()
        .into_iter()
        .rev()
        .zip(10..13u32)
        .step_by(2)
        .last()
        .map(|(d, n)| d.0 * 100 + n);
    println!("last {last:?}");
    let set: HashSet<u32> = ds()
        .into_iter()
        .rev()
        .zip(10..13u32)
        .step_by(2)
        .map(|(d, n)| d.0 * 100 + n)
        .collect();
    let mut set: Vec<u32> = set.into_iter().collect();
    set.sort();
    println!("set {set:?}");
    let n = ds().into_iter().rev().enumerate().take(3).count();
    println!("taken {n}");
    let n = ds().into_iter().rev().enumerate().take(9).count();
    println!("all taken {n}");
}
