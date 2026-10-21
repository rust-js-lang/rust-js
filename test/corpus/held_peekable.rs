// A `Peekable` of a `Fuse` a struct holds, as itertools' `WithPosition`
// does: its source and what it peeked at kept, so its `Debug` and a clone,
// where it is, are Rust's (ADR 0543).
use std::fmt;
use std::iter::{Fuse, Peekable};

struct WithPosition<I: Iterator> {
    handled_first: bool,
    peekable: Peekable<Fuse<I>>,
}

impl<I> Clone for WithPosition<I>
where
    I: Iterator + Clone,
    I::Item: Clone,
{
    fn clone(&self) -> Self {
        WithPosition { handled_first: self.handled_first, peekable: self.peekable.clone() }
    }
}

fn with_position<I: Iterator>(iter: I) -> WithPosition<I> {
    WithPosition { handled_first: false, peekable: iter.fuse().peekable() }
}

impl<I: Iterator> Iterator for WithPosition<I> {
    type Item = (&'static str, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let item = self.peekable.next()?;
        let first = !self.handled_first;
        self.handled_first = true;
        let last = self.peekable.peek().is_none();
        Some((match (first, last) {
            (true, true) => "only",
            (true, false) => "first",
            (false, true) => "last",
            (false, false) => "middle",
        }, item))
    }
}

impl<I: Iterator> fmt::Debug for WithPosition<I>
where
    Peekable<Fuse<I>>: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("WithPosition")
            .field("handled_first", &self.handled_first)
            .field("peekable", &self.peekable)
            .finish()
    }
}

// One of an array's iterator, which shows what it peeked at.
#[derive(Debug)]
struct Lexer {
    chars: Peekable<std::vec::IntoIter<char>>,
}

fn main() {
    let mut words = with_position(vec!["a", "b", "c"].into_iter());
    println!("{words:?}");
    println!("{:?} {:?}", words.peekable.size_hint(), words.next());
    println!("{words:?} {:?}", words.peekable.size_hint());
    let copy = words.clone();
    println!("{:?}", words.collect::<Vec<_>>());
    println!("{copy:#?}");
    println!("{:?}", copy.collect::<Vec<_>>());
    let mut lexer = Lexer { chars: vec!['x', 'y'].into_iter().peekable() };
    lexer.chars.peek();
    println!("{lexer:?} {:?}", lexer.chars.size_hint());
    let mut one = with_position(1..2);
    println!("{:?} {:?} {one:?}", one.next(), one.next());
}
