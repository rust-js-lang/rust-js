// A `find` of the crate's iterator whose own `Drop` panics, taken by
// value: the source is dropped after what it found is given back, so as
// its drop panics the found item, a local of the caller's by then, is
// dropped as the panic unwinds (`$findDropping`'s `$closeBare`, #2572).
use std::panic::{self, AssertUnwindSafe};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
struct L(u32);

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// The crate's iterator, whose own `Drop` says so and panics.
struct Src<T> {
    items: std::vec::IntoIter<T>,
}

impl<T> Iterator for Src<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        self.items.next()
    }
}

impl<T> Drop for Src<T> {
    fn drop(&mut self) {
        println!("drop src");
        if !std::thread::panicking() {
            panic!("src");
        }
    }
}

fn src<T>(items: Vec<T>) -> Src<T> {
    Src {
        items: items.into_iter(),
    }
}

fn main() {
    let made = panic::catch_unwind(AssertUnwindSafe(|| {
        let hit = src(vec![L(1), L(2), L(3)]).find(|x| x.0 == 2);
        println!("{hit:?}");
    }));
    println!("{}", made.is_err());
}
