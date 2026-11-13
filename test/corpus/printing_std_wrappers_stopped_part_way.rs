//@ library-differs
// `{:?}` of std's wrappers, `Reverse`, `OnceCell`, `OnceLock`, `LazyCell`,
// `Bound` and `Poll`, of the ranges, and of a slice's, a `Vec`'s and an
// array's iterators, one of whose parts prints and then panics, the panic
// caught: Rust's stdout keeps what was written before the part, `x
// Reverse(!Grid(`, as std's `debug_tuple` writes each piece as it's taken,
// and a range's `start..end`, which std writes straight to the formatter,
// `!Grid(1)..!Grid(` (#1530). A writer's `Err` keeps it too, in what it gave
// back. `Wrapping`'s and a `Cow`'s `Debug` is their part's own.
use std::borrow::Cow;
use std::cell::{LazyCell, OnceCell};
use std::cmp::Reverse;
use std::fmt::{self, Debug, Write};
use std::num::{Saturating, Wrapping};
use std::ops::{Bound, Range, RangeInclusive};
use std::panic::catch_unwind;
use std::sync::OnceLock;
use std::task::Poll;

#[derive(Clone)]
struct Grid(u32);

impl fmt::Debug for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        print!("!");
        write!(f, "Grid(")?;
        if self.0 > 2 {
            panic!("too big");
        }
        write!(f, "{})", self.0)
    }
}

#[derive(Clone)]
struct Failing(u32);

impl fmt::Debug for Failing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failing(")?;
        if self.0 > 2 {
            return Err(fmt::Error);
        }
        write!(f, "{})", self.0)
    }
}

fn once(n: u32) -> OnceCell<Grid> {
    let cell = OnceCell::new();
    if n > 0 {
        let _ = cell.set(Grid(n));
    }
    cell
}

fn locked<T>(value: T) -> OnceLock<T> {
    let lock = OnceLock::new();
    let _ = lock.set(value);
    lock
}

fn shown(what: &str, show: impl Fn() + std::panic::UnwindSafe) {
    let caught = catch_unwind(show);
    println!("{what} caught {}", caught.is_err());
}

fn failed(text: String, written: fmt::Result) {
    println!("{written:?} {text:?}");
}

fn show_range(range: &Range<Grid>) {
    println!("range {range:?} {range:#?} end");
}

fn show_inclusive(range: &RangeInclusive<Grid>) {
    println!("inclusive {range:?} end");
}

fn reversed<T: Debug>(value: T) -> String {
    let mut text = String::new();
    match write!(text, "{:?}", Reverse(value)) {
        Ok(()) => text,
        Err(_) => format!("err {text}"),
    }
}

fn excluded<T: Debug>(value: T) -> String {
    let mut text = String::new();
    match write!(text, "{:#?}", (Bound::Excluded(value), Poll::<u8>::Pending)) {
        Ok(()) => text,
        Err(_) => format!("err {text}"),
    }
}

fn main() {
    // One part, which stops.
    shown("a", || println!("a {:?} b", Reverse(Grid(3))));
    shown("c", || println!("c {:#?} d", Reverse(Grid(3))));
    shown("e", || println!("e {:?} f", once(3)));
    shown("g", || println!("g {:#?} h", locked(Grid(3))));
    shown("i", || println!("i {:?} j", Bound::Included(Grid(3))));
    shown("k", || println!("k {:#?} l", Bound::Excluded(Grid(3))));
    shown("m", || println!("m {:?} n", Poll::Ready(Grid(3))));
    shown("o", || println!("o {:#?} p", Poll::Ready(Grid(3))));
    shown("q", || println!("q {:?} r", LazyCell::new(|| Grid(3))));
    shown("q2", || {
        let lazy = LazyCell::new(|| Grid(3));
        LazyCell::force(&lazy);
        println!("q2 {lazy:#?} r2");
    });

    // The first, the middle and the last of three.
    shown("s", || {
        println!("s {:?} t", [Reverse(Grid(3)), Reverse(Grid(1)), Reverse(Grid(2))])
    });
    shown("u", || println!("u {:#?} v", vec![once(1), once(3), once(0)]));
    shown("w", || {
        println!(
            "w {:?} x",
            [Bound::Unbounded, Bound::Included(Grid(1)), Bound::Excluded(Grid(3))]
        )
    });
    shown("y", || {
        println!(
            "y {:?} z",
            vec![Poll::Pending, Poll::Ready(Grid(3)), Poll::Ready(Grid(1))]
        )
    });

    // The ranges: the start stops, the end, and one of three.
    shown("A", || println!("A {:?} B", Grid(3)..Grid(1)));
    shown("C", || println!("C {:?} D", Grid(1)..Grid(3)));
    shown("E", || println!("E {:#?} F", Grid(1)..=Grid(3)));
    shown("G", || println!("G {:?} H", Grid(3)..));
    shown("I", || println!("I {:?} J", ..Grid(3)));
    shown("K", || println!("K {:#?} L", ..=Grid(3)));
    shown("M", || {
        println!("M {:?} N", vec![Grid(1)..Grid(2), Grid(1)..Grid(3), Grid(2)..Grid(1)])
    });
    shown("O", || println!("O {:#?} P", vec![Grid(1)..Grid(2), Grid(3)..Grid(1)]));
    shown("Q", || println!("Q {:?} R", Some(Reverse(Grid(1)..Grid(3)))));
    shown("S", || {
        println!("S {:#?} T", (Grid(1)..Grid(2), Bound::Included(Grid(1)..=Grid(3))))
    });

    // None stops: each shown whole.
    shown("U", || {
        println!("U {:?} {:#?} V", Reverse(Grid(1)), (Grid(1)..Grid(2), once(0)))
    });
    shown("W", || {
        println!(
            "W {:?} {:#?} X",
            (Bound::<Grid>::Unbounded, Poll::<Grid>::Pending),
            ..Grid(2)
        )
    });

    // A range given by reference, and one changed after it's made.
    shown("Y", || show_range(&(Grid(1)..Grid(3))));
    shown("Z", || show_range(&(Grid(1)..Grid(2))));
    shown("0", || show_inclusive(&(Grid(3)..=Grid(1))));
    let mut range = Grid(1)..Grid(2);
    range.end = Grid(3);
    shown("1", move || println!("1 {range:?} end"));

    // The iterators that show what they have left, `as_slice()`.
    shown("2", || println!("2 {:?} 3", vec![Grid(3), Grid(1)].into_iter()));
    shown("4", || println!("4 {:#?} 5", [Grid(1), Grid(3), Grid(2)].into_iter()));
    shown("6", || {
        let grids = [Grid(1), Grid(2), Grid(3)];
        let mut it = grids.iter();
        it.next();
        println!("6 {it:?} 7");
    });
    shown("8", || {
        let mut grids = vec![Grid(1), Grid(2), Grid(3), Grid(1)];
        let mut drain = grids.drain(1..);
        drain.next_back();
        println!("8 {drain:#?} 9");
    });
    shown("q", || println!("q {:?} r", Some([Grid(1)].iter())));

    // Those whose `Debug` is their part's.
    shown("k", || println!("k {:?} l", (Wrapping(Grid(1)), Saturating(Grid(3)))));
    shown("m", || {
        println!("m {:?} n", vec![Cow::Owned(Grid(1)), Cow::Borrowed(&Grid(3))])
    });
    shown("o", || println!("o {:#?} p", Wrapping(vec![Grid(1), Grid(3)])));

    // Generic code's, of a part given.
    println!("{}", reversed(Failing(3)));
    println!("{}", reversed(Failing(1)..Failing(2)));
    println!("{}", reversed(5u8..=7));
    println!("{}", excluded(Failing(1)..Failing(3)));
    println!("{}", excluded(Failing(2)));

    // A writer's `Err`: what came before it kept.
    let mut text = String::new();
    let written = write!(text, "a {:?}", Reverse(Failing(3)));
    failed(text, written);
    let mut text = String::new();
    let written = write!(text, "b {:#?}", locked(Failing(3)));
    failed(text, written);
    let mut text = String::new();
    let written = write!(text, "c {:?}", Failing(1)..Failing(3));
    failed(text, written);
    let mut text = String::new();
    let written = write!(text, "d {:?}", Failing(3)..=Failing(1));
    failed(text, written);
    let mut text = String::new();
    let written = write!(
        text,
        "e {:#?}",
        [Bound::Excluded(Failing(1)), Bound::Included(Failing(3))]
    );
    failed(text, written);
    let mut text = String::new();
    let written = write!(
        text,
        "f {:?}",
        vec![Poll::Ready(Failing(1)..Failing(2)), Poll::Ready(Failing(3)..Failing(1))]
    );
    failed(text, written);
    let mut text = String::new();
    let written = write!(text, "g {:?}", ..=Failing(1));
    failed(text, written);
    let mut steps = 1u8..=2;
    for _ in steps.by_ref() {}
    let mut text = String::new();
    let written = write!(text, "h {:?}", (steps, Reverse(Failing(3))));
    failed(text, written);
    let mut text = String::new();
    let written = write!(text, "i {:?}", vec![Failing(1), Failing(3)].into_iter());
    failed(text, written);
    let mut text = String::new();
    let written = write!(text, "j {:#?}", [Failing(3)].iter());
    failed(text, written);
    let mut text = String::new();
    let written = write!(
        text,
        "k {:?}",
        (Wrapping(Failing(1)), Cow::<Failing>::Owned(Failing(3)))
    );
    failed(text, written);
}
