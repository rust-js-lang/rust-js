// A `Vec`'s `splice(range, replacement)` is std's `Splice`: what it took
// out, given as it's stepped; dropped, what it hasn't given dropped, then
// its replacement stepped into the gap, what's put there staying where a
// step panics, and the replacement dropped last; collected, held, asked its
// `len()` or stepped from its back before it's dropped; and of the crate's
// own iterator, stepped by its `next` (#1147, #1149).
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl Clone for D {
    fn clone(&self) -> D {
        println!("clone {}", self.0);
        D(self.0 + 100)
    }
}

/// The crate's own iterator, which prints its steps and its drop.
struct It<I>(I);

impl<I: Iterator> Iterator for It<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        println!("next");
        self.0.next()
    }
}

impl<I> Drop for It<I> {
    fn drop(&mut self) {
        println!("drop it");
    }
}

fn made() -> impl Iterator<Item = D> {
    (1..3).map(|n| {
        println!("step {n}");
        D(n)
    })
}

fn generic<F: Fn() -> I, I: Iterator<Item = D>>(v: &mut Vec<D>, make: F) {
    v.splice(0..1, make());
}

fn main() {
    let mut v = vec![D(10), D(11), D(12), D(13)];
    // Collected, then put in place as it's dropped.
    let removed: Vec<D> = v.splice(1..3, made()).collect();
    println!("{removed:?} {v:?}");
    drop(removed);
    // Held: what it hasn't given dropped where it's dropped, then filled.
    let mut s = v.splice(0..2, made());
    println!("{:?}", s.next());
    println!("held");
    drop(s);
    println!("{v:?}");
    let mut s = v.splice(0..3, made());
    println!("{:?} {}", s.next_back(), s.len());
    println!("{:?}", s.next());
    println!("between");
    drop(s);
    println!("{v:?}");
    // Dropped at once, of the crate's own iterator, of a clone, and of one
    // whose step panics: what's put stays, the tail put back.
    v.splice(0..1, It(vec![D(1)].into_iter()));
    println!("{v:?}");
    v.splice(1..2, std::iter::repeat_n(D(7), 2));
    println!("{v:?}");
    let r = catch_unwind(AssertUnwindSafe(|| {
        v.splice(1..2, vec![D(4), D(5), D(6)].into_iter().map(|d| if d.0 == 6 { panic!("at 6") } else { d }));
    }));
    println!("{} {v:?}", r.is_err());
    // Of one whose hint is no help, a filter's: the gap put in place as it's
    // stepped, what's past it collected apart, dropped where a step panics.
    let r = catch_unwind(AssertUnwindSafe(|| {
        v.splice(
            1..3,
            vec![D(30), D(31), D(32), D(33)].into_iter().filter(|d| if d.0 == 33 { panic!("at 33") } else { true }),
        );
    }));
    println!("{} {v:?}", r.is_err());
    // At the end, with no tail: pushed as each is stepped.
    let r = catch_unwind(AssertUnwindSafe(|| {
        let at = v.len() - 1;
        v.splice(at.., It(vec![D(8), D(9)].into_iter().map(|d| if d.0 == 9 { panic!("at 9") } else { d })));
    }));
    println!("{} {v:?}", r.is_err());
    generic(&mut v, || It(vec![D(20), D(21)].into_iter()));
    println!("{v:?}");
}
