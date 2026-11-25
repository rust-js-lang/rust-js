//@ run-fail: drop
// Generic code's fallible `sum` into a `Result`, of a source whose own
// `Drop` panics, drops the kept `Err` as that panic unwinds: std's
// `try_process` frame holds it while the source, in its `GenericShunt`, is
// dropped, so the source is the consumer's to close, never dropped after
// the call (#2580, #2572).
#[derive(Debug)]
struct L(u32);

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

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

fn summed<I: Iterator<Item = Result<u32, L>>>(it: I) -> Result<u32, L> {
    it.sum()
}

fn main() {
    let made = summed(source(vec![Ok(1), Err(L(2)), Ok(3)], true, None));
    println!("{made:?}");
}
