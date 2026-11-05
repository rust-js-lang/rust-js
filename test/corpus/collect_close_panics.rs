// A collect into a `Vec`, which drops what it steps once it's done: where
// that drop panics, a `take()`'s of a flatten whose source holds an item
// whose drop panics, or a zip's, the `Vec` it collected is dropped as it
// unwinds, after what that drop dropped, as std's `from_iter` drops the one
// it holds (#677).
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

fn main() {
    let r = catch_unwind(AssertUnwindSafe(|| {
        let v: Vec<D> = vec![D(1), D(2), D(3), D(4), D(5)]
            .into_iter()
            .map(|x| vec![D(x.0 + 10), x])
            .flatten()
            .take(3)
            .collect();
        println!("{}", v.len());
    }));
    println!("{}", r.is_err());
    let r = catch_unwind(AssertUnwindSafe(|| {
        let v: Vec<(D, u32)> = vec![D(1), D(2), D(3), D(4), D(5)]
            .into_iter()
            .rev()
            .map(|x| vec![D(x.0 + 10), x])
            .flatten()
            .zip(10..13u32)
            .collect();
        println!("{}", v.len());
    }));
    println!("{}", r.is_err());
    // And where a step panics, what it collected, then what's left (#422).
    let r = catch_unwind(AssertUnwindSafe(|| {
        let v: Vec<D> = vec![D(4), D(5), D(6), D(7)]
            .into_iter()
            .filter(|d| {
                if d.0 == 6 {
                    panic!("filter");
                }
                true
            })
            .collect();
        println!("{}", v.len());
    }));
    println!("{}", r.is_err());
}
