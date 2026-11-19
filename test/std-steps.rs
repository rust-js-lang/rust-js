//! std's steps, as std itself takes them: each adapter and wrapper of
//! `core::iter` and `alloc`, over sources that print each step of
//! `Iterator`, `DoubleEndedIterator` and `ExactSizeIterator` they write as
//! they're asked it, with its argument, `src.nth(1)`. Each step of each is
//! asked once of a fresh one, in each of `SIZES`' runs over each kind of
//! source, and what its sources were asked is one line's cell, `Skip#0
//! |fold: src.nth(0) src.fold`, `Take#bare0` of the `bare` kind's. test/std-steps.test.ts runs it natively, unstable features on
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

/// A source that writes every step but the folds, `fold`, `try_fold`,
/// `rfold` and `try_rfold`, which are the traits' defaults over its others.
#[derive(Clone)]
struct NoFold<T>(Loud<T>);

impl<T> Iterator for NoFold<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
    fn nth(&mut self, n: usize) -> Option<T> {
        self.0.nth(n)
    }
    fn advance_by(&mut self, n: usize) -> Result<(), NonZero<usize>> {
        self.0.advance_by(n)
    }
    fn count(self) -> usize {
        self.0.count()
    }
    fn last(self) -> Option<T> {
        self.0.last()
    }
}

impl<T> DoubleEndedIterator for NoFold<T> {
    fn next_back(&mut self) -> Option<T> {
        self.0.next_back()
    }
    fn nth_back(&mut self, n: usize) -> Option<T> {
        self.0.nth_back(n)
    }
    fn advance_back_by(&mut self, n: usize) -> Result<(), NonZero<usize>> {
        self.0.advance_back_by(n)
    }
}

impl<T> ExactSizeIterator for NoFold<T> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// A source that writes only what it must, `next`, `next_back` and
/// `size_hint`, every other step the traits' defaults over them.
#[derive(Clone)]
struct Bare<T>(Loud<T>);

impl<T> Iterator for Bare<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

impl<T> DoubleEndedIterator for Bare<T> {
    fn next_back(&mut self) -> Option<T> {
        self.0.next_back()
    }
}

impl<T> ExactSizeIterator for Bare<T> {}

/// A kind of source: what it is of `T`, and its label in a run's name,
/// none of the one that writes every step.
trait Kind {
    type Of<T: Clone + 'static>: DoubleEndedIterator<Item = T> + ExactSizeIterator + Clone + 'static;
    const LABEL: &'static str;
    fn of<T: Clone + 'static>(name: &'static str, items: Vec<T>) -> Self::Of<T>;

    /// The source an adapter holds, of the run's first length.
    fn src() -> Self::Of<u32> {
        Self::of("src", (1..=sizes().0 as u32).collect())
    }

    /// A second source, a chain's or a zip's, of its second length.
    fn other() -> Self::Of<u32> {
        Self::of("other", (6..6 + sizes().1 as u32).collect())
    }

    /// A source of references, a `copied()`'s or a `cloned()`'s.
    fn refs() -> Self::Of<&'static u32> {
        let items: &'static [u32] = Box::leak((1..=sizes().0 as u32).collect::<Vec<_>>().into_boxed_slice());
        Self::of("src", items.iter().collect())
    }

    /// A source of sources, a flatten's: up to two inner ones, of the
    /// run's second length.
    fn nested() -> Self::Of<Self::Of<u32>> {
        let (outer, inner) = sizes();
        let inners = (0..outer.min(2)).map(|_| Self::of("inner", (1..=inner as u32).collect()));
        Self::of("src", inners.collect())
    }

    /// A `&mut` of a source, leaked, as each step takes one by value.
    fn lent() -> &'static mut Self::Of<u32> {
        Box::leak(Box::new(Self::src()))
    }
}

struct All;
impl Kind for All {
    type Of<T: Clone + 'static> = Loud<T>;
    const LABEL: &'static str = "";
    fn of<T: Clone + 'static>(name: &'static str, items: Vec<T>) -> Loud<T> {
        Loud { name, items }
    }
}

struct NoFolds;
impl Kind for NoFolds {
    type Of<T: Clone + 'static> = NoFold<T>;
    const LABEL: &'static str = "nofold";
    fn of<T: Clone + 'static>(name: &'static str, items: Vec<T>) -> NoFold<T> {
        NoFold(Loud { name, items })
    }
}

struct Bares;
impl Kind for Bares {
    type Of<T: Clone + 'static> = Bare<T>;
    const LABEL: &'static str = "bare";
    fn of<T: Clone + 'static>(name: &'static str, items: Vec<T>) -> Bare<T> {
        Bare(Loud { name, items })
    }
}

/// The sources' lengths in each run: both long, the first empty, the
/// second empty, and one item each, so a step whose path turns on what's
/// left, a chain's `nth` past its first side, is seen down each.
const SIZES: [(usize, usize); 4] = [(5, 3), (0, 3), (5, 0), (1, 1)];

thread_local! {
    static RUN: Cell<usize> = const { Cell::new(0) };
    static KIND: Cell<&'static str> = const { Cell::new("") };
}

fn sizes() -> (usize, usize) {
    SIZES[RUN.get()]
}

/// Each of `Iterator`'s steps asked of `$make`, a fresh one each, by
/// `Iterator`'s path, so a `&mut dyn`'s is the `&mut`'s impl's, not the
/// `dyn`'s: what its sources were asked, a cell each.
macro_rules! front {
    ($name:expr, $make:expr) => {
        front!($name, $make, ["next", "size_hint", "nth", "advance_by", "fold", "try_fold", "last", "count"])
    };
    ($name:expr, $make:expr, $steps:expr) => {{
        print!("{}#{}{}", $name, KIND.get(), RUN.get());
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
        print!("{}#{}{}", $name, KIND.get(), RUN.get());
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
        print!("{}#{}{}", $name, KIND.get(), RUN.get());
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

fn probe<K: Kind>() {
    KIND.set(K::LABEL);
    for run in 0..SIZES.len() {
        RUN.set(run);
        every!("Map", K::src().map(|x| x));
        ends!("Filter", K::src().filter(|x| x % 2 == 1));
        ends!("FilterMap", K::src().filter_map(Some));
        every!("Enumerate", K::src().enumerate());
        every!("Skip", K::src().skip(1));
        every!("Take", K::src().take(3));
        every!("StepBy", K::src().step_by(2));
        ends!("Chain", K::src().chain(K::other()));
        every!("Zip", K::src().zip(K::other()));
        every!("Rev", K::src().rev());
        every!("Fuse", K::src().fuse());
        every!("Peekable", K::src().peekable());
        ends!("Flatten", K::nested().flatten());
        ends!("FlatMap", K::nested().flat_map(|x| x));
        every!("Inspect", K::src().inspect(|_| ()));
        every!("Copied", K::refs().copied());
        every!("Cloned", K::refs().cloned());
        front!("TakeWhile", K::src().take_while(|x| *x < 4));
        front!("SkipWhile", K::src().skip_while(|x| *x < 2));
        front!("MapWhile", K::src().map_while(Some));
        front!("Scan", K::src().scan(0, |_, x| Some(x)));
        // Endless: none of the steps that consume it whole.
        front!("Cycle", K::src().cycle(), ["next", "size_hint", "nth", "advance_by", "try_fold"]);
        every!("&mut I", K::lent());
        ends!("&mut dyn I", K::lent() as &mut dyn DoubleEndedIterator<Item = u32>);
        exact!("&mut dyn I", K::lent() as &mut dyn ExactSizeIterator<Item = u32>);
        every!("Box<I>", Box::new(K::src()));
        ends!("Box<dyn I>", Box::new(K::src()) as Box<dyn DoubleEndedIterator<Item = u32>>);
        exact!("Box<dyn I>", Box::new(K::src()) as Box<dyn ExactSizeIterator<Item = u32>>);
    }
}

fn main() {
    probe::<All>();
    probe::<NoFolds>();
    probe::<Bares>();
}
