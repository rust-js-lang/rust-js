// A `cloned()` is each item its source steps, cloned as it's stepped, as
// rust-js makes it, never std's `Cloned` struct: std's `Cloned` calls the
// item's `Clone` once for each item it gives, from either end, and none
// for an item it skips by `len()` or a hint, which the hand form does too
// (ADR 1136, kept, proved equal). Here, with what an identity can't show:
// an item whose own `Clone` prints and counts, so the number and order of
// its calls are seen, from the front, from the back, of a partly stepped
// one asked its `len()`; and an item with a destructor, each clone dropped
// where Rust drops it.
use std::cell::Cell;

thread_local! {
    static CLONES: Cell<u32> = const { Cell::new(0) };
}

#[derive(Debug)]
struct Loud(u32);

impl Clone for Loud {
    fn clone(&self) -> Self {
        CLONES.with(|c| c.set(c.get() + 1));
        println!("clone {}", self.0);
        Loud(self.0)
    }
}

#[derive(Debug)]
struct Noted(&'static str);

impl Clone for Noted {
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
        Noted(self.0)
    }
}

impl Drop for Noted {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn clones() -> u32 {
    CLONES.with(|c| c.get())
}

fn last_of<I: DoubleEndedIterator>(mut it: I) -> Option<I::Item> {
    it.next_back()
}

fn main() {
    let v = vec![Loud(1), Loud(2), Loud(3), Loud(4), Loud(5)];

    // From the front, then the back, of one held.
    let mut it = v.iter().cloned();
    println!(
        "{:?} {:?} len {} clones {}",
        it.next(),
        it.next_back(),
        it.len(),
        clones()
    );
    // Partly stepped, the rest collected.
    let rest: Vec<Loud> = it.collect();
    println!("{:?} clones {}", rest, clones());

    // Its length and hint ask no clone.
    let it = v.iter().cloned();
    println!("len {} hint {:?} clones {}", it.len(), it.size_hint(), clones());

    // Reversed, and given to generic code that steps it from its back.
    let back: Vec<Loud> = v.iter().cloned().rev().take(2).collect();
    println!("{:?} clones {}", back, clones());
    println!("{:?} clones {}", last_of(v.iter().cloned()), clones());

    // Skipped and counted: `nth` clones only the item it gives.
    println!("{:?} clones {}", v.iter().cloned().nth(3), clones());
    println!("count {} clones {}", v.iter().cloned().count(), clones());

    // An item with a destructor: each clone dropped where Rust drops it.
    let notes = [Noted("a"), Noted("b"), Noted("c")];
    {
        let mut it = notes.iter().cloned();
        let first = it.next();
        println!("first {:?}", first);
        let last = it.next_back();
        println!("last {:?}", last);
        println!("left {}", it.len());
    }
    println!("after the block");
    let kept: Vec<Noted> = notes.iter().cloned().filter(|n| n.0 != "b").collect();
    println!("kept {:?}", kept);
}
