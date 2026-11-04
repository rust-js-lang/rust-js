// An `enumerate()`'s `count()` is its source's, as std's `Enumerate::count`
// is. Of a `peekable()` that peeked, that's std's `Peekable::count`, which
// keeps the item it peeked at till its source's `count` ends, then drops
// it: after what its source drops, as that `count` ends or unwinds (#655).
use std::panic::catch_unwind;

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// The crate's iterator of `D`s up to 5, whose `next` panics at `boom`.
struct Items {
    at: u32,
    boom: u32,
}

impl Items {
    fn new(boom: u32) -> Self {
        Items { at: 1, boom }
    }
}

impl Drop for Items {
    fn drop(&mut self) {
        println!("drop items");
    }
}

impl Iterator for Items {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        if self.at == 6 {
            return None;
        }
        if self.at == self.boom {
            panic!("next");
        }
        self.at += 1;
        Some(D(self.at - 1))
    }
}

fn main() {
    // What it peeked at is dropped after its source's rest and its source.
    let mut it = Items::new(0).peekable();
    println!("peeked {:?}", it.peek().map(|d| d.0));
    println!("count {}", it.enumerate().count());

    // As the source's `next` panics, too: its source unwinds first.
    let ended = catch_unwind(|| {
        let mut it = Items::new(3).peekable();
        println!("peeked {:?}", it.peek().map(|d| d.0));
        let n = it.enumerate().count();
        println!("count {n}");
    });
    println!("panicked {}", ended.is_err());

    // One that peeked at nothing yet is its source's count alone.
    println!("count {}", Items::new(0).peekable().enumerate().count());
}
