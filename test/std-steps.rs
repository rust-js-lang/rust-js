//! std's steps, as std itself takes them: each adapter and wrapper of
//! `core::iter` and `alloc`, over sources that write every step of
//! `Iterator`, `DoubleEndedIterator` and `ExactSizeIterator` and print each
//! as they're asked it, with its argument, `src.nth(1)`. Each step of each
//! is asked once of a fresh one, in each of `SIZES`' runs, and what its
//! sources were asked is one line's cell, `Skip#0 |fold: src.nth(0)
//! src.fold`. test/std-steps.test.ts runs it natively, unstable features on
//! (`try_fold`, `advance_by` and `is_empty` are written here as std's own
//! sources write them), and writes what it prints as the catalog,
//! src/lower/recognition/std_steps.txt (#2112).
//!
//! Nothing here is rust-js's: it's run natively alone, the oracle.

#![feature(try_trait_v2, iter_advance_by, exact_size_is_empty)]

use std::cell::Cell;
use std::num::NonZero;
use std::ops::Try;

/// A source that writes every step, each printing its name, its argument
/// where it has one, as it's asked it.
#[derive(Clone)]
struct Loud<T> {
    name: &'static str,
    items: Vec<T>,
}

fn say(name: &str, step: &str) {
    print!(" {name}.{step}");
}

impl<T> Iterator for Loud<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        say(self.name, "next");
        if self.items.is_empty() { None } else { Some(self.items.remove(0)) }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        say(self.name, "size_hint");
        (self.items.len(), Some(self.items.len()))
    }
    fn nth(&mut self, n: usize) -> Option<T> {
        say(self.name, &format!("nth({n})"));
        if n < self.items.len() {
            self.items.drain(..n);
            Some(self.items.remove(0))
        } else {
            self.items.clear();
            None
        }
    }
    fn advance_by(&mut self, n: usize) -> Result<(), NonZero<usize>> {
        say(self.name, &format!("advance_by({n})"));
        let k = n.min(self.items.len());
        self.items.drain(..k);
        NonZero::new(n - k).map_or(Ok(()), Err)
    }
    fn fold<B, F: FnMut(B, T) -> B>(self, init: B, mut f: F) -> B {
        say(self.name, "fold");
        let mut a = init;
        for x in self.items {
            a = f(a, x);
        }
        a
    }
    fn try_fold<B, F: FnMut(B, T) -> R, R: Try<Output = B>>(&mut self, init: B, mut f: F) -> R {
        say(self.name, "try_fold");
        let mut a = init;
        while !self.items.is_empty() {
            a = f(a, self.items.remove(0))?;
        }
        R::from_output(a)
    }
    fn count(self) -> usize {
        say(self.name, "count");
        self.items.len()
    }
    fn last(mut self) -> Option<T> {
        say(self.name, "last");
        self.items.pop()
    }
}

impl<T> DoubleEndedIterator for Loud<T> {
    fn next_back(&mut self) -> Option<T> {
        say(self.name, "next_back");
        self.items.pop()
    }
    fn nth_back(&mut self, n: usize) -> Option<T> {
        say(self.name, &format!("nth_back({n})"));
        let keep = self.items.len().saturating_sub(n);
        self.items.truncate(keep);
        self.items.pop()
    }
    fn advance_back_by(&mut self, n: usize) -> Result<(), NonZero<usize>> {
        say(self.name, &format!("advance_back_by({n})"));
        let k = n.min(self.items.len());
        self.items.truncate(self.items.len() - k);
        NonZero::new(n - k).map_or(Ok(()), Err)
    }
    fn rfold<B, F: FnMut(B, T) -> B>(self, init: B, mut f: F) -> B {
        say(self.name, "rfold");
        let mut a = init;
        for x in self.items.into_iter().rev() {
            a = f(a, x);
        }
        a
    }
    fn try_rfold<B, F: FnMut(B, T) -> R, R: Try<Output = B>>(&mut self, init: B, mut f: F) -> R {
        say(self.name, "try_rfold");
        let mut a = init;
        while let Some(x) = self.items.pop() {
            a = f(a, x)?;
        }
        R::from_output(a)
    }
}

impl<T> ExactSizeIterator for Loud<T> {
    fn len(&self) -> usize {
        say(self.name, "len");
        self.items.len()
    }
    fn is_empty(&self) -> bool {
        say(self.name, "is_empty");
        self.items.is_empty()
    }
}

/// The sources' lengths in each run: both long, the first empty, the
/// second empty, and one item each, so a step whose path turns on what's
/// left, a chain's `nth` past its first side, is seen down each.
const SIZES: [(usize, usize); 4] = [(5, 3), (0, 3), (5, 0), (1, 1)];

thread_local! {
    static RUN: Cell<usize> = const { Cell::new(0) };
}

fn sizes() -> (usize, usize) {
    SIZES[RUN.get()]
}

/// The source an adapter holds, of the run's first length.
fn src() -> Loud<u32> {
    Loud { name: "src", items: (1..=sizes().0 as u32).collect() }
}

/// A second source, a chain's or a zip's, of its second length.
fn other() -> Loud<u32> {
    Loud { name: "other", items: (6..6 + sizes().1 as u32).collect() }
}

/// A source of references, a `copied()`'s or a `cloned()`'s.
fn refs() -> Loud<&'static u32> {
    let items: &'static [u32] = Box::leak((1..=sizes().0 as u32).collect::<Vec<_>>().into_boxed_slice());
    Loud { name: "src", items: items.iter().collect() }
}

/// A source of sources, a flatten's: up to two inner ones, of the run's
/// second length.
fn nested() -> Loud<Loud<u32>> {
    let (outer, inner) = sizes();
    let inners = (0..outer.min(2)).map(|_| Loud { name: "inner", items: (1..=inner as u32).collect() });
    Loud { name: "src", items: inners.collect() }
}

/// A `&mut` of a source, leaked, as each step takes one by value.
fn lent() -> &'static mut Loud<u32> {
    Box::leak(Box::new(src()))
}

/// Each of `Iterator`'s steps asked of `$make`, a fresh one each, by
/// `Iterator`'s path, so a `&mut dyn`'s is the `&mut`'s impl's, not the
/// `dyn`'s: what its sources were asked, a cell each.
macro_rules! front {
    ($name:expr, $make:expr) => {
        front!($name, $make, ["next", "size_hint", "nth", "advance_by", "fold", "try_fold", "last", "count"])
    };
    ($name:expr, $make:expr, $steps:expr) => {{
        print!("{}#{}", $name, RUN.get());
        for step in $steps {
            print!(" |{step}:");
            let mut it = $make;
            match step {
                "next" => drop(Iterator::next(&mut it)),
                "size_hint" => drop(Iterator::size_hint(&it)),
                "nth" => drop(Iterator::nth(&mut it, 1)),
                "advance_by" => drop(Iterator::advance_by(&mut it, 2)),
                "fold" => drop(Iterator::fold(it, 0u32, |a, _| a + 1)),
                "try_fold" => drop(Iterator::try_fold(&mut it, 0u32, |a, _| if a == 1 { None } else { Some(a + 1) })),
                "last" => drop(Iterator::last(it)),
                "count" => drop(Iterator::count(it)),
                _ => unreachable!(),
            }
        }
        println!();
    }};
}

/// Each of `DoubleEndedIterator`'s, likewise.
macro_rules! back {
    ($name:expr, $make:expr) => {{
        print!("{}#{}", $name, RUN.get());
        for step in ["next_back", "nth_back", "advance_back_by", "rfold", "try_rfold"] {
            print!(" |{step}:");
            let mut it = $make;
            match step {
                "next_back" => drop(DoubleEndedIterator::next_back(&mut it)),
                "nth_back" => drop(DoubleEndedIterator::nth_back(&mut it, 1)),
                "advance_back_by" => drop(DoubleEndedIterator::advance_back_by(&mut it, 2)),
                "rfold" => drop(DoubleEndedIterator::rfold(it, 0u32, |a, _| a + 1)),
                "try_rfold" => drop(DoubleEndedIterator::try_rfold(&mut it, 0u32, |a, _| if a == 1 { None } else { Some(a + 1) })),
                _ => unreachable!(),
            }
        }
        println!();
    }};
}

/// Each of `ExactSizeIterator`'s, likewise.
macro_rules! exact {
    ($name:expr, $make:expr) => {{
        print!("{}#{}", $name, RUN.get());
        for step in ["len", "is_empty"] {
            print!(" |{step}:");
            let it = $make;
            match step {
                "len" => drop(ExactSizeIterator::len(&it)),
                "is_empty" => drop(ExactSizeIterator::is_empty(&it)),
                _ => unreachable!(),
            }
        }
        println!();
    }};
}

/// Each of the three traits' steps it has.
macro_rules! every {
    ($name:expr, $make:expr) => {{
        front!($name, $make);
        back!($name, $make);
        exact!($name, $make);
    }};
}

/// Of both ends, not exact-sized.
macro_rules! ends {
    ($name:expr, $make:expr) => {{
        front!($name, $make);
        back!($name, $make);
    }};
}

fn main() {
    for run in 0..SIZES.len() {
        RUN.set(run);
        every!("Map", src().map(|x| x));
        ends!("Filter", src().filter(|x| x % 2 == 1));
        ends!("FilterMap", src().filter_map(Some));
        every!("Enumerate", src().enumerate());
        every!("Skip", src().skip(1));
        every!("Take", src().take(3));
        every!("StepBy", src().step_by(2));
        ends!("Chain", src().chain(other()));
        every!("Zip", src().zip(other()));
        every!("Rev", src().rev());
        every!("Fuse", src().fuse());
        every!("Peekable", src().peekable());
        ends!("Flatten", nested().flatten());
        ends!("FlatMap", nested().flat_map(|x| x));
        every!("Inspect", src().inspect(|_| ()));
        every!("Copied", refs().copied());
        every!("Cloned", refs().cloned());
        front!("TakeWhile", src().take_while(|x| *x < 4));
        front!("SkipWhile", src().skip_while(|x| *x < 2));
        front!("MapWhile", src().map_while(Some));
        front!("Scan", src().scan(0, |_, x| Some(x)));
        // Endless: none of the steps that consume it whole.
        front!("Cycle", src().cycle(), ["next", "size_hint", "nth", "advance_by", "try_fold"]);
        every!("&mut I", lent());
        ends!("&mut dyn I", lent() as &mut dyn DoubleEndedIterator<Item = u32>);
        exact!("&mut dyn I", lent() as &mut dyn ExactSizeIterator<Item = u32>);
        every!("Box<I>", Box::new(src()));
        ends!("Box<dyn I>", Box::new(src()) as Box<dyn DoubleEndedIterator<Item = u32>>);
        exact!("Box<dyn I>", Box::new(src()) as Box<dyn ExactSizeIterator<Item = u32>>);
    }
}
