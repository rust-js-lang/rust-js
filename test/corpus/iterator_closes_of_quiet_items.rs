// Closing an iterator, `it.return()`, never throws where nothing it may
// hold has a drop that may panic (#861): a `Peekable` of items whose drop
// only prints drops in a `finally`. Where what it peeked at, or an inner
// adapter's peeked item, may panic as it drops, the close may throw, so it
// drops after the `try` and as a panic unwinds, and a peeked item that
// panics as it's dropped still drops what's after it.
use std::cell::Cell;
use std::panic;

thread_local! {
    static BOMB: Cell<u32> = const { Cell::new(0) };
}

struct Quiet(u32);

impl Drop for Quiet {
    fn drop(&mut self) {
        println!("quiet {}", self.0);
    }
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        if BOMB.get() == self.0 {
            panic!("loud {} panics", self.0);
        }
        println!("loud {}", self.0);
    }
}

fn work(n: u32) -> u32 {
    if n > 100 {
        panic!("too big: {n}");
    }
    n + 1
}

fn peeked_quiet(n: u32) -> u32 {
    let mut it = vec![Quiet(1), Quiet(2), Quiet(3)].into_iter().peekable();
    let first = it.peek().map_or(0, |q| q.0);
    work(n) + first
}

fn peeked_loud(n: u32) -> u32 {
    let mut it = vec![Loud(1), Loud(2), Loud(3)].into_iter().peekable();
    let first = it.peek().map_or(0, |l| l.0);
    work(n) + first
}

fn mapped_peeked_loud(n: u32) -> u32 {
    let mut inner = vec![Loud(4), Loud(5)].into_iter().peekable();
    let first = inner.peek().map_or(0, |l| l.0);
    let mut it = inner.map(|l| l.0 * 10);
    let second = it.next().unwrap_or(0);
    work(n) + first + second
}

/// The crate's iterator of `Loud`s: what its `Peekable` peeks at is of its
/// `Item`, which its type doesn't name.
struct Louds(u32);

impl Iterator for Louds {
    type Item = Loud;
    fn next(&mut self) -> Option<Loud> {
        self.0 += 1;
        (self.0 <= 3).then(|| Loud(self.0 + 5))
    }
}

fn peeked_own(n: u32) -> u32 {
    let mut it = Louds(0).peekable();
    let first = it.peek().map_or(0, |l| l.0);
    work(n) + first
}

/// Of a `map()` of it, whose `Item` is a number: what the inner one
/// peeked at is still held.
fn mapped_own(n: u32) -> u32 {
    let mut inner = Louds(0).peekable();
    let first = inner.peek().map_or(0, |l| l.0);
    let mut it = inner.map(|l| l.0 * 10);
    let second = it.next().unwrap_or(0);
    work(n) + first + second
}

fn main() {
    println!("mapped own {}", mapped_own(8));
    let r = panic::catch_unwind(|| mapped_own(800));
    println!("mapped own caught {}", r.is_err());

    println!("own {}", peeked_own(6));
    let r = panic::catch_unwind(|| peeked_own(600));
    println!("own caught {}", r.is_err());
    BOMB.set(6);
    let r = panic::catch_unwind(|| peeked_own(7));
    println!("own bomb caught {}", r.is_err());
    BOMB.set(0);

    println!("peeked quiet {}", peeked_quiet(1));
    let r = panic::catch_unwind(|| peeked_quiet(200));
    println!("peeked quiet caught {}", r.is_err());

    println!("peeked loud {}", peeked_loud(2));
    let r = panic::catch_unwind(|| peeked_loud(300));
    println!("peeked loud caught {}", r.is_err());
    BOMB.set(1);
    let r = panic::catch_unwind(|| peeked_loud(3));
    println!("peeked loud bomb caught {}", r.is_err());
    BOMB.set(0);

    println!("mapped {}", mapped_peeked_loud(4));
    let r = panic::catch_unwind(|| mapped_peeked_loud(400));
    println!("mapped caught {}", r.is_err());
    BOMB.set(5);
    let r = panic::catch_unwind(|| mapped_peeked_loud(5));
    println!("mapped bomb caught {}", r.is_err());
}
