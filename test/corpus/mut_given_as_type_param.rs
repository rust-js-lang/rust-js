// A `&mut` given as a type parameter's value is a box, as one given as a
// `&mut T` is (ADR 0180): copied back after the call, and what's in it
// taken out where the call gives it back; an impl for `&mut T` reads
// through it, std's `Deref` and `fmt::Write` of a `&mut` too (ADR 0454).
use std::fmt::{self, Write};
use std::ops::Deref;

trait Total {
    fn total(&self) -> u32;
}

impl Total for Vec<u32> {
    fn total(&self) -> u32 {
        self.iter().sum()
    }
}

impl<T: Total + ?Sized> Total for &mut T {
    fn total(&self) -> u32 {
        (**self).total() + 100
    }
}

fn totalled<D: Total>(items: D) -> u32 {
    items.total()
}

fn same<D>(given: D) -> D {
    given
}

/// # Safety
/// What it derefs to stays put.
unsafe trait StableDeref: Deref {}

unsafe impl<T: ?Sized> StableDeref for &mut T {}

fn first<D: StableDeref<Target = Vec<u32>>>(items: D) -> u32 {
    items[0]
}

struct Counter {
    text: String,
}

impl Write for Counter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.text.push_str(s);
        Ok(())
    }
}

fn render<W: Write>(mut out: W, n: u32) -> fmt::Result {
    write!(out, "<{n}>")
}

fn twice<W: Write>(out: &mut W, n: u32) -> fmt::Result {
    render(&mut *out, n)?;
    render(out, n + 1)
}

fn main() {
    let mut items = vec![1, 2];
    println!("{}", totalled(&mut items));
    println!("{}", (&mut items).total());
    same(&mut items).push(3);
    println!("{items:?} {}", first(&mut items));
    let mut counter = Counter { text: String::new() };
    render(&mut counter, 1).unwrap();
    twice(&mut counter, 2).unwrap();
    let mut text = String::new();
    twice(&mut text, 5).unwrap();
    println!("{} {text}", counter.text);
}
