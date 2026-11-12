// Generic code's `c.extend(it)` of a `C: Extend<A>` drops the iterator it
// was given as std's `extend` does: a `Vec`'s and a `VecDeque`'s step it by
// `next` and drop it at the end, `drop Up 3`; a list's, a hash set's and a
// B-tree's take it by `fold`, the crate's own where it writes one, which
// drops it, `fold` then `drop Up 0`; and a step that panics drops what's
// left as it unwinds. A crate's own `Extend` drops the iterator it's given,
// not what it was made of (#1511).
use std::collections::{BTreeSet, HashSet, LinkedList, VecDeque};
use std::panic::catch_unwind;

/// With a `fold` of its own.
struct Up {
    n: u32,
}

impl Iterator for Up {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.n < 3 {
            self.n += 1;
            Some(self.n)
        } else {
            None
        }
    }

    fn fold<B, F: FnMut(B, u32) -> B>(self, init: B, mut f: F) -> B {
        println!("fold");
        (self.n + 1..=3).fold(init, |acc, x| f(acc, x))
    }
}

impl Drop for Up {
    fn drop(&mut self) {
        println!("drop Up {}", self.n);
    }
}

/// With none: std's `fold`, by `next`, which takes it and drops it.
struct Down {
    n: u32,
}

impl Iterator for Down {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        assert!(self.n != 7, "seven");
        if self.n > 0 {
            self.n -= 1;
            Some(self.n)
        } else {
            None
        }
    }
}

impl Drop for Down {
    fn drop(&mut self) {
        println!("drop Down {}", self.n);
    }
}

struct Bag(Vec<u32>);

impl Extend<u32> for Bag {
    fn extend<T: IntoIterator<Item = u32>>(&mut self, iter: T) {
        for x in iter {
            self.0.push(x);
        }
    }
}

fn up<C: Extend<u32>>(mut c: C) -> C {
    c.extend(Up { n: 0 });
    c
}

fn down<C: Extend<u32>>(mut c: C, n: u32) -> C {
    c.extend(Down { n });
    c
}

fn main() {
    println!("vec {}", up(Vec::new()).len());
    println!("deque {}", up(VecDeque::new()).len());
    println!("list {}", up(LinkedList::new()).len());
    println!("set {}", up(HashSet::new()).len());
    println!("btree {}", up(BTreeSet::new()).len());
    println!("bag {}", up(Bag(vec![])).0.len());
    println!("vec {}", down(Vec::new(), 2).len());
    println!("list {}", down(LinkedList::new(), 2).len());
    println!("set {}", down(HashSet::new(), 2).len());
    println!("bag {}", down(Bag(vec![]), 2).0.len());
    let r = catch_unwind(|| down(Vec::new(), 7).len());
    println!("caught {}", r.is_err());
    let r = catch_unwind(|| down(LinkedList::new(), 7).len());
    println!("caught {}", r.is_err());
}
