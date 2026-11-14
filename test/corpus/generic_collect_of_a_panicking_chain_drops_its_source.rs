// A collect in generic code of what its caller gives, an adapter chain of
// the crate's own iterator with a destructor whose closure panics: the
// caller's iterator dropped as the panic unwinds, as std's `from_iter`
// drops what it's given, into a `String` and into a `Vec` (#1753). Generic
// code holds it as the caller's closing, which owns its drop (ADR 0190).
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

struct Own(Vec<D>);

impl Iterator for Own {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }
}

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop Own of {}", self.0.len());
    }
}

fn collected<I: Iterator, B: FromIterator<I::Item>>(it: I) -> B {
    it.collect()
}

fn main() {
    let text = catch_unwind(AssertUnwindSafe(|| {
        let _text: String = collected(Own(vec![D(5), D(6), D(7)]).skip(1).map(|d| {
            assert!(d.0 != 6, "map");
            d.0.to_string()
        }));
    }));
    println!("{}", text.is_err());
    let numbers = catch_unwind(AssertUnwindSafe(|| {
        let _numbers: Vec<u32> = collected(Own(vec![D(1), D(2), D(3)]).map(|d| {
            assert!(d.0 != 2, "map");
            d.0
        }));
    }));
    println!("{}", numbers.is_err());
    let whole: String = collected(Own(vec![D(8), D(9)]).map(|d| d.0.to_string()));
    println!("{whole}");
}
