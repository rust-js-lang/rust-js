// A `Debug` builder's `entries(items)` is std's `for entry in items {
// self.entry(&entry) }`: each entry shown, then dropped, and dropped as a
// panic showing it unwinds; what's left of `items` dropped where a step or
// a show panics, the crate's own iterator with a `Drop` at its end; and of
// generic code's iterator, `F: Fn() -> I`'s, the caller's own value,
// stepped by its `Iterator` (#1147, #1148).
use std::fmt;
use std::panic;

struct D(u32);

impl fmt::Debug for D {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0 == 13 {
            panic!("unlucky");
        }
        write!(f, "D({})", self.0)
    }
}

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
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

struct List(Vec<u32>);

impl fmt::Debug for List {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.0.iter().map(|&n| D(n))).finish()
    }
}

struct Owned(fn() -> Vec<D>);

impl fmt::Debug for Owned {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_set().entries((self.0)()).finish()
    }
}

struct Stepped(fn() -> It<std::vec::IntoIter<D>>);

impl fmt::Debug for Stepped {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries((self.0)()).finish()
    }
}

struct Pairs(fn() -> Vec<(u32, D)>);

impl fmt::Debug for Pairs {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries((self.0)()).finish()
    }
}

/// Generic code's: the iterator `F` makes, whatever it is.
struct Listed<F>(F);

impl<F, I> fmt::Debug for Listed<F>
where
    F: Fn() -> I,
    I: Iterator,
    I::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries((self.0)()).finish()
    }
}

fn shown(value: impl fmt::Debug + panic::RefUnwindSafe) {
    match panic::catch_unwind(|| format!("{value:?}")) {
        Ok(text) => println!("{text}"),
        Err(_) => println!("panicked"),
    }
}

fn main() {
    shown(List(vec![1, 2]));
    shown(Owned(|| vec![D(3), D(4)]));
    shown(Stepped(|| It(vec![D(5), D(6)].into_iter())));
    shown(Pairs(|| vec![(1, D(7)), (2, D(8))]));
    shown(Listed(|| It(vec![D(9), D(10)].into_iter())));
    shown(Listed(|| vec![1u32, 2].into_iter()));
    // A show that panics: the entry dropped as it unwinds, then what's left.
    shown(Owned(|| vec![D(11), D(13), D(12)]));
    shown(Stepped(|| It(vec![D(13), D(14)].into_iter())));
}
