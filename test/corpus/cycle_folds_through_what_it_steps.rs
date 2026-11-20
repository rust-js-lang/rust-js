// `cycle()` folds and passes over what it steps as std's `Cycle` does
// (#2192): its `try_fold` is what it steps' own, so a `skip` inside asks the
// crate's `nth(n - 1)`, and from its back `nth_back(n - 1)`, through a
// `fuse` too; its `advance_by` is what it steps' own, so `nth` and a `skip`
// of it ask no `nth` of the crate's; a fold begun mid-round folds what's
// left first; an empty one ends, folded or passed over; and where a clone
// panics inside its fold, the closure's captures drop, then the
// accumulator, as its frame drops its parameters.
use std::cell::Cell;
use std::panic::catch_unwind;

thread_local! {
    static CLONES: Cell<u32> = const { Cell::new(0) };
    static PANIC_CLONE_AT: Cell<u32> = const { Cell::new(u32::MAX) };
}

struct Count(u32);

impl Clone for Count {
    fn clone(&self) -> Self {
        let n = CLONES.get() + 1;
        CLONES.set(n);
        if n == PANIC_CLONE_AT.get() {
            panic!("clone {n}");
        }
        Count(self.0)
    }
}

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 3 {
            return None;
        }
        self.0 += 1;
        Some(self.0)
    }

    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("own nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}

impl DoubleEndedIterator for Count {
    fn next_back(&mut self) -> Option<u32> {
        if self.0 == 3 {
            return None;
        }
        self.0 += 1;
        Some(4 - self.0)
    }

    fn nth_back(&mut self, n: usize) -> Option<u32> {
        println!("own nth_back {n}");
        for _ in 0..n {
            self.next_back()?;
        }
        self.next_back()
    }
}

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let n = Count(0).fuse().skip(1).cycle().take(4).count();
    println!("count {n}");
    let folded = Count(0).skip(1).cycle().take(5).fold(0, |a, x| a * 10 + x);
    println!("fold {folded}");
    println!("any {}", Count(0).skip(1).cycle().any(|x| x == 3));
    println!("position {:?}", Count(0).skip(1).cycle().position(|x| x == 2));
    println!("nth {:?}", Count(0).skip(1).cycle().nth(4));
    println!("skip {:?}", Count(0).skip(1).cycle().skip(3).next());
    let back = Count(0).rev().skip(1).cycle().take(4);
    println!("rev {}", back.fold(0, |a, x| a * 10 + x));
    let mut mid = Count(0).skip(1).cycle();
    mid.next();
    println!("mid {}", mid.take(4).fold(0, |a, x| a * 10 + x));
    println!("empty {}", Count(3).skip(1).cycle().take(4).count());
    println!("empty nth {:?}", Count(3).skip(1).cycle().nth(2));
    println!("empty any {}", Count(3).skip(1).cycle().any(|x| x == 2));
    println!("range {:?}", (0..3u32).skip(1).cycle().nth(5));

    // The second clone `cycle()` makes inside its fold panics: the fold's
    // closure, which owns `f`, drops first, then the accumulator.
    CLONES.set(0);
    PANIC_CLONE_AT.set(3);
    let caught = catch_unwind(|| {
        let f = Loud("f");
        let acc = (Loud("acc"), 0);
        Count(0).skip(1).cycle().take(9).fold(acc, move |(held, a), x| {
            let _ = &f;
            (held, a + x)
        })
    });
    PANIC_CLONE_AT.set(u32::MAX);
    println!("caught {}", caught.is_err());
}
