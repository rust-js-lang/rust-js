//@ library-differs
// std's iterator adapters' `Debug` written into a writer a piece at a
// time, as std writes them (#1712): a `Fuse`'s, an `Enumerate`'s and a
// `Peekable`'s by `@rust-js/std`'s compiled bodies, framed so a writer arms
// them, and a `Cycle`'s, which the plain build holds as the runtime's, by
// its `Cycle { orig: .., iter: .. }`; each of an array's and a `Vec`'s
// iterator, `IntoIter([1, 2])`, by what it has left. Through a derived
// `Debug` and a builder of the crate's own, `{:#?}` too, and printed.
use std::fmt::{self, Write};
use std::iter::{Cycle, Enumerate, Fuse, Peekable};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

#[derive(Debug)]
struct Holder<I: Iterator> {
    fused: Fuse<I>,
    counted: Enumerate<I>,
}

struct Shown<I: Iterator + Clone>(Cycle<I>, Peekable<I>);

impl<I: Iterator + Clone + fmt::Debug> fmt::Debug for Shown<I>
where
    I::Item: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Shown").field(&self.0).field(&self.1).finish()
    }
}

fn holder<I: Iterator + Clone>(it: I) -> Holder<I> {
    Holder {
        fused: it.clone().fuse(),
        counted: it.enumerate(),
    }
}

fn written<I: Iterator + Clone + fmt::Debug>(log: &mut Log, it: I)
where
    I::Item: fmt::Debug,
{
    let mut h = holder(it.clone());
    write!(log, "{:?}", h).unwrap();
    h.counted.next();
    write!(log, "|{:#?}", h).unwrap();
    let mut peeked = it.clone().peekable();
    write!(log, "|{:?}", Shown(it.clone().cycle(), it.clone().peekable())).unwrap();
    peeked.peek();
    write!(log, "|{:?}", Shown(it.cycle(), peeked)).unwrap();
}

fn main() {
    let mut log = Log(Vec::new());
    written(&mut log, vec![1, 2].into_iter());
    written(&mut log, [10u8, 20].into_iter());
    println!("{:?}", log.0);
    println!("{:?}", holder([5].into_iter()));
}
