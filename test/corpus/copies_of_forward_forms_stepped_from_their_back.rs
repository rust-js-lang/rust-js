// A `cloned()` or a `copied()` stepped from its back, of a source whose own
// form steps only forward, a `step_by(n)`'s `values().filter(..)`, a
// `fuse()`'s or a `peekable()`'s: its source made knowing its back too, as a
// map's is, `$mapEnds($stepByEnds(..), clone)`. It was `$mapEnds` of the
// forward form, which threw `it.back is not a function` where Rust folds
// from the back (#851).
struct D(u32);

impl Clone for D {
    fn clone(&self) -> D {
        println!("clone {}", self.0);
        D(self.0)
    }
}

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let items = vec![D(1), D(2), D(3), D(4), D(5)];
    let stepped = items.iter().step_by(2).cloned().rfold(0, |a, d| a * 10 + d.0);
    println!("step_by {stepped}");
    let fused = items.iter().fuse().cloned().rfold(0, |a, d| a * 10 + d.0);
    println!("fuse {fused}");
    let peeked = items.iter().peekable().cloned().rfold(0, |a, d| a * 10 + d.0);
    println!("peekable {peeked}");
    let numbers = [1u32, 2, 3, 4, 5, 6, 7];
    let copied = numbers.iter().step_by(3).copied().rfold(0, |a, n| a * 10 + n);
    println!("copied {copied}");
    let found = numbers.iter().step_by(2).copied().rfind(|n| n % 3 == 0);
    println!("{found:?}");
}
