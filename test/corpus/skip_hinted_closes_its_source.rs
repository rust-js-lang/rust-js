// A `skip(n)` generic code asks the hint of is made with its hint, std's
// `Skip`'s of its source's (#1236), and its source closed to drop what it
// holds: so where a drop of what's left panics, the rest are still dropped,
// as `IntoIter`'s drop goes on (#1244).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == 3 {
            panic!("drop");
        }
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4), D(5)]
}

fn hinted<I: Iterator<Item = D>>(it: I) -> usize {
    it.size_hint().0
}

fn main() {
    println!("hint {}", hinted(vec![D(10), D(11)].into_iter().skip(1)));
    let r = catch_unwind(AssertUnwindSafe(|| ds().into_iter().skip(1).last().map(|d| d.0 + 0)));
    println!("skipped {:?}", r.ok());
    let r = catch_unwind(AssertUnwindSafe(|| ds().into_iter().take(4).last().map(|d| d.0 + 0)));
    println!("taken {:?}", r.ok());
}
