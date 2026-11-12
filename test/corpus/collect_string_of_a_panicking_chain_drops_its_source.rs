// A `String` collected of an adapter chain whose closure panics, or whose
// item's drop at the closure's end does, drops the chain's source, a
// struct iterator and what it holds, as the panic unwinds, as a `Vec`'s
// collect does: the fold std routes a `String` through closes it (#1610).
use std::panic::{AssertUnwindSafe, catch_unwind};
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop D{}", self.0);
        if self.0 == 2 && !std::thread::panicking() {
            panic!("drop of 2");
        }
    }
}
struct Own(Vec<D>);
impl Iterator for Own {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        Some(self.0.remove(0))
    }
}
impl Drop for Own {
    fn drop(&mut self) {
        println!("drop Own");
    }
}
fn main() {
    let skipped = catch_unwind(AssertUnwindSafe(|| {
        let text: String = Own(vec![D(1), D(2), D(3), D(4)])
            .take(3)
            .skip(1)
            .map(|d| d.0.to_string())
            .collect();
        text
    }));
    println!("{}", skipped.is_err());
    let asserted = catch_unwind(AssertUnwindSafe(|| {
        let text: String = Own(vec![D(5), D(6), D(7)])
            .skip(1)
            .map(|d| {
                assert!(d.0 != 6, "map");
                d.0.to_string()
            })
            .collect();
        text
    }));
    println!("{}", asserted.is_err());
}
