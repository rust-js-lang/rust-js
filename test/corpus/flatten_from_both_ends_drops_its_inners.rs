// A `flatten()` stepped from its back drops what it holds as std's
// `FlattenCompat` does, field by field: its source, then the inner iterator
// it's stepping from the front, then the one from the back, each though an
// earlier one's drop panics (#705).
use std::cell::Cell;
use std::panic::catch_unwind;

thread_local! {
    /// The item whose drop panics, or none.
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        if self.0 == BOOM.get() {
            BOOM.set(0);
            panic!("drop {}", self.0);
        }
    }
}

fn ds(n: u32) -> Vec<D> {
    (1..=n).map(D).collect()
}

/// The crate's iterator of `[D(10 * n), D(n)]`s, which prints as it's
/// dropped.
struct Source(u32, u32);

impl Drop for Source {
    fn drop(&mut self) {
        println!("drop source");
    }
}

impl Iterator for Source {
    type Item = Vec<D>;
    fn next(&mut self) -> Option<Vec<D>> {
        (self.0 < self.1).then(|| {
            self.0 += 1;
            vec![D(self.0 * 10), D(self.0)]
        })
    }
}

impl DoubleEndedIterator for Source {
    fn next_back(&mut self) -> Option<Vec<D>> {
        (self.0 < self.1).then(|| {
            self.1 -= 1;
            vec![D((self.1 + 1) * 10), D(self.1 + 1)]
        })
    }
}

fn main() {
    // The back inner's rest is dropped as it's dropped, after its source's.
    let last = ds(2).into_iter().map(|d| vec![D(d.0 * 10), d]).flatten().next_back();
    println!("last {:?}", last.as_ref().map(|d| d.0));
    drop(last);

    // Though its source's rest's drop panics, as drop glue goes on.
    let ended = catch_unwind(|| {
        BOOM.set(1);
        let last = ds(3).into_iter().map(|d| vec![D(d.0 * 10), d]).flatten().next_back();
        println!("last {:?}", last.map(|d| d.0));
    });
    println!("panicked {}", ended.is_err());

    // Of the crate's iterator, which it drops before its back inner too.
    let last = Source(0, 3).flatten().next_back();
    println!("last {:?}", last.map(|d| d.0));

    // Reversed, part-way.
    let firsts: Vec<u32> = Source(0, 3).flatten().rev().take(3).map(|d| d.0).collect();
    println!("firsts {firsts:?}");

    // Whose back inner's rest's drop panics.
    let ended = catch_unwind(|| {
        BOOM.set(30);
        let firsts: Vec<u32> = Source(0, 3).flatten().rev().take(1).map(|d| d.0).collect();
        println!("firsts {firsts:?}");
    });
    println!("panicked {}", ended.is_err());
}
