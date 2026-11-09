// What two iterators compared, `eq`, `ne`, `cmp` and the comparisons, drop
// and when, as std's `iter_compare` does: the item taken of the first
// dropped before either iterator, the argument then the receiver, at the
// end, not as either's steps end, and as a panic in the argument's step
// unwinds; the crate's own iterator with a `Drop` closed once, by `eq` and
// by a `collect()` into an `Option` (#1144, #1145, #1146).
use std::panic;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// The crate's own iterator, which prints as it's dropped, and panics at
/// its `step`th step.
struct It<I> {
    items: I,
    name: &'static str,
    steps: u32,
    step: u32,
}

fn it<I>(items: I, name: &'static str, step: u32) -> It<I> {
    It { items, name, steps: 0, step }
}

impl<I: Iterator> Iterator for It<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.steps += 1;
        if self.steps == self.step {
            panic!("step");
        }
        self.items.next()
    }
}

impl<I> Drop for It<I> {
    fn drop(&mut self) {
        println!("drop {}", self.name);
    }
}

fn caught(f: impl FnOnce() + panic::UnwindSafe) {
    match panic::catch_unwind(f) {
        Ok(()) => println!("ended"),
        Err(_) => println!("panicked"),
    }
}

fn main() {
    // The crate's own iterator compared, closed once.
    println!("{}", it(vec![D(1), D(2)].into_iter(), "a", 0).eq(vec![D(1), D(3)]));
    println!("{}", it(vec![D(1)].into_iter(), "a", 0).ne(vec![D(1), D(4)]));
    // Std's compared with the crate's, which ends first: the item taken of
    // the first dropped before the argument is.
    println!("{}", vec![D(1), D(2)].into_iter().eq(it(vec![D(1)].into_iter(), "b", 0)));
    // The argument's step panics: the item taken dropped as it unwinds,
    // then the argument, then what's left of the receiver.
    caught(|| {
        println!("{}", vec![D(1), D(2), D(3)].into_iter().eq(it(vec![D(1)].into_iter(), "b", 2)));
    });
    // `cmp` and the comparisons drop what they're given.
    println!("{:?}", vec![1u32, 2].into_iter().cmp(it(vec![1u32, 3].into_iter(), "b", 0)));
    println!("{:?}", it(vec![1u32].into_iter(), "a", 0).partial_cmp(vec![1u32, 2]));
    println!("{}", (1..3u32).lt(it(vec![1u32, 3].into_iter(), "b", 0)));
    println!("{}", (1..3u32).ge(it(vec![1u32, 3].into_iter(), "b", 0)));
    // Collected into an `Option`, closed once.
    let all: Option<Vec<D>> = it(vec![Some(D(1)), Some(D(2))].into_iter(), "a", 0).collect();
    println!("{all:?}");
    drop(all);
    let none: Option<Vec<D>> = it(vec![Some(D(1)), None, Some(D(3))].into_iter(), "a", 0).collect();
    println!("{none:?}");
}
