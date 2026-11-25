// Generic code's fallible `product` into an `Option`, of a source whose
// `next` panics part way, drops the source as the panic unwinds, though its
// items have no destructor: the consumer closes what it takes (#2580,
// #2572).
use std::panic::{self, AssertUnwindSafe};

/// A source whose own `Drop` says so, and panics where it's told.
struct Source<T> {
    items: std::vec::IntoIter<T>,
    panics_on_drop: bool,
    panics_on_step: Option<usize>,
    stepped: usize,
}

impl<T> Iterator for Source<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        println!("next src");
        self.stepped += 1;
        if self.panics_on_step == Some(self.stepped) {
            panic!("step");
        }
        self.items.next()
    }
}

impl<T> Drop for Source<T> {
    fn drop(&mut self) {
        println!("drop src");
        if self.panics_on_drop && !std::thread::panicking() {
            panic!("drop");
        }
    }
}

fn source<T>(items: Vec<T>, panics_on_drop: bool, panics_on_step: Option<usize>) -> Source<T> {
    Source {
        items: items.into_iter(),
        panics_on_drop,
        panics_on_step,
        stepped: 0,
    }
}

fn multiplied<I: Iterator<Item = Option<u32>>>(it: I) -> Option<u32> {
    it.product()
}

fn main() {
    let made = panic::catch_unwind(AssertUnwindSafe(|| {
        multiplied(source(vec![Some(2), Some(3), Some(4)], false, Some(2)))
    }));
    println!("{}", made.is_err());
}
