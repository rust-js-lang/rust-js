//@ library-differs
// A trait's default lowered into a blanket impl's dictionary takes what its
// callers give as they give it: `other` of `then_all<J: IntoIterator>(self,
// other: J) -> Both<Self, J::IntoIter>`, held in the struct it gives back,
// is given as it is with its `IntoIterator` dictionary, which makes its
// iterator; its body called `$intoIter(other)`, as though it were given a
// JS iterator, and read the tuple struct `Three(0)`, `[0]`, as an array, so
// it gave `[2, 3]` where Rust gives `[2, 3, 1, 2, 3]`. And a unit struct's
// iterator, `undefined`, given as it is, is stepped, asked its hint and
// held as any other is; `$given`, `$sizeHint` and `$held` read a property
// of it and threw (itertools' `interleave_shortest`, `zip_eq` and
// `get` of its laziness tests).
use std::cell::Cell;

/// Gives what's after its number up to 3.
struct Three(u8);

impl Iterator for Three {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        (self.0 < 3).then(|| {
            self.0 += 1;
            self.0
        })
    }
}

thread_local! {
    static LEFT: Cell<u8> = const { Cell::new(0) };
}

/// Gives what's left in a thread-local, a unit struct.
struct Countdown;

impl Iterator for Countdown {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        let left = LEFT.get();
        (left > 0).then(|| {
            LEFT.set(left - 1);
            left
        })
    }
}

/// Gives none, a unit struct whose hint says so.
#[derive(Clone)]
struct Never;

impl Iterator for Never {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        None
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(0))
    }
}

struct Both<I, J> {
    a: I,
    b: J,
}

impl<I: Iterator<Item = u8>, J: Iterator<Item = u8>> Iterator for Both<I, J> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        self.a.next().or_else(|| self.b.next())
    }
}

fn both<I: Iterator, J>(a: I, b: J) -> Both<I, J> {
    Both { a, b }
}

trait Tools: Iterator {
    fn then_all<J: IntoIterator<Item = Self::Item>>(self, other: J) -> Both<Self, J::IntoIter>
    where
        Self: Sized,
    {
        both(self, other.into_iter())
    }

    fn firsts(self) -> Vec<Self::Item>
    where
        Self: Sized,
    {
        self.take(2).collect()
    }

    fn hinted(&self) -> (usize, Option<usize>) {
        self.size_hint()
    }
}

impl<T: Iterator> Tools for T {}

fn zipped<I: IntoIterator, J: IntoIterator>(i: I, j: J) -> Vec<(I::Item, J::Item)> {
    i.into_iter().zip(j).collect()
}

fn main() {
    println!("{:?}", Three(1).then_all(Three(0)).collect::<Vec<_>>());
    println!("{:?}", Never.then_all(Three(2)).collect::<Vec<_>>());
    println!("{:?}", Three(0).then_all(Never).collect::<Vec<_>>());
    println!("{:?}", Never.then_all(vec![4, 5]).collect::<Vec<_>>());
    LEFT.set(3);
    println!("{:?}", Countdown.then_all(Three(1)).collect::<Vec<_>>());

    LEFT.set(4);
    println!("{:?} {:?}", Countdown.firsts(), Never.firsts());
    println!("{:?} {:?}", Never.hinted(), Countdown.hinted());
    LEFT.set(2);
    println!("{:?}", zipped(Countdown, [7, 8, 9]));
    println!("{:?}", zipped(Never.clone(), Three(0)));
}
