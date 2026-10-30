//@ library-differs
// A `Cell` of an `Option` of what could look like `None`, `Option<u8>`'s or
// a type parameter's: `take`, `replace` and `get` give what it held as it's
// kept, `Some(None)` apart from `None`, as Rust's do (ADR 0537).
use std::cell::Cell;

struct Format<I> {
    inner: Cell<Option<I>>,
}

impl<I: Clone> Clone for Format<I> {
    fn clone(&self) -> Self {
        let inner = self.inner.take();
        let copy = inner.clone();
        self.inner.set(inner);
        Self { inner: Cell::new(copy) }
    }
}

fn main() {
    let words = Format { inner: Cell::new(Some(vec!["a"])) };
    let copy = words.clone();
    println!("{:?} {:?}", words.inner.take(), copy.inner.take());
    let nested: Format<Option<u8>> = Format { inner: Cell::new(Some(None)) };
    let copy = nested.clone();
    println!("{:?} {:?}", nested.inner.take(), copy.inner.take());
    let cell: Cell<Option<Option<u8>>> = Cell::new(Some(Some(1)));
    println!("{:?}", cell.get());
    println!("{:?} {:?}", cell.replace(Some(None)), cell.get());
    println!("{:?} {:?}", cell.take(), cell.take());
}
