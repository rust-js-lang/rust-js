// A box of the crate's iterator, chained or skipped where it's made, is
// made in place, `$lazyChain` and `$skipping`, which ask what std's `Chain`
// and `Skip` ask: a chain's `last` asks each side's own `last`, the first's
// though the second has none; its `count` under a `skip` is each side's own
// `count`; a skip's `last` is its source's own `last` after what it skips.
// Where a side's `last` or `count`, or what a skip passes over, panics, what
// it holds is dropped as it unwinds, the other side and the source with it,
// as std drops the `Chain` or `Skip` it was given by value (#647, #735,
// #2214).
use std::panic::catch_unwind;

/// Counts down from its number; its own `last` and `count` print, and
/// where it `panics`, they panic, as its `next` does at its last.
struct Own(u32, bool);

impl Iterator for Own {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.1 && self.0 == 1 {
            panic!("own next");
        }
        self.0 = self.0.checked_sub(1)?;
        Some(self.0)
    }

    fn last(self) -> Option<u32> {
        println!("own last {}", self.0);
        if self.1 {
            panic!("own last");
        }
        if self.0 == 0 {
            None
        } else {
            Some(0)
        }
    }

    fn count(self) -> usize {
        println!("own count {}", self.0);
        if self.1 {
            panic!("own count");
        }
        self.0 as usize
    }
}

impl Drop for Own {
    fn drop(&mut self) {
        println!("drop own {}", self.0);
    }
}

fn main() {
    println!("{:?}", Box::new(Own(3, false)).chain(Own(2, false)).last());
    println!("{:?}", Box::new(Own(3, false)).chain(Own(0, false)).last());
    let r = catch_unwind(|| Box::new(Own(3, true)).chain(Own(2, false)).last());
    println!("{}", r.is_err());
    let r = catch_unwind(|| Box::new(Own(3, true)).chain(Own(2, false)).count());
    println!("{}", r.is_err());
    println!("{}", Box::new(Own(3, false)).chain(Own(2, false)).skip(1).count());
    println!("{:?}", Box::new(Own(3, false)).skip(1).last());
    let r = catch_unwind(|| Box::new(Own(3, true)).skip(3).last());
    println!("{}", r.is_err());
}
