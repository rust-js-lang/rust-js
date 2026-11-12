// Two iterators' `eq` drops each pair it compared, `b`'s item first, though
// its `PartialEq` or one of those drops panics: the other dropped as the
// panic unwinds, then what's left of each closed, as std's `iter_compare`
// closure drops them (#1609).
use std::panic::{AssertUnwindSafe, catch_unwind};
struct D(u32, &'static str);
impl PartialEq for D {
    fn eq(&self, other: &D) -> bool {
        assert!(self.0 != 2, "eq of 2");
        self.0 == other.0
    }
}
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}{} panicking={}", self.1, self.0, std::thread::panicking());
        if self.0 == 1 && self.1 == "b" && !std::thread::panicking() {
            panic!("drop of b1");
        }
    }
}
fn main() {
    let r = catch_unwind(AssertUnwindSafe(|| {
        vec![D(1, "a"), D(3, "a")].into_iter().eq(vec![D(1, "b"), D(3, "b")])
    }));
    println!("{}", r.is_err());
    let r = catch_unwind(AssertUnwindSafe(|| {
        vec![D(2, "a"), D(3, "a")].into_iter().eq(vec![D(2, "b"), D(3, "b")])
    }));
    println!("{}", r.is_err());
}
