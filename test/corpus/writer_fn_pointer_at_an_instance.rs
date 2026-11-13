//@ library-differs
// A writer function generic over a type, named as a `fn` pointer at one of
// its instances, called through the pointer: its JS takes no `Formatter`,
// nor does the pointer's, so what it's given is its own, and its
// dictionary after (#1710). So does one whose forms differ from its callers' (#1187).
use std::fmt;

trait Is<T> {
    fn is(self) -> T;
}

impl<T> Is<T> for T {
    fn is(self) -> T {
        self
    }
}

type Writing = fn(&mut fmt::Formatter<'_>, i32) -> fmt::Result;

fn bracketed(f: &mut fmt::Formatter<'_>, n: impl Is<i32>) -> fmt::Result {
    let n: i32 = Is::is(n);
    write!(f, "[{n}]")
}

// Generic code's form of a `&mut` to a number, a box, differs from its
// callers', so its pointer at `i32` adapts what it's given (#1187).
fn bumped<T: fmt::Display>(f: &mut fmt::Formatter<'_>, n: &mut T) -> fmt::Result {
    write!(f, "<{}>", n)
}

struct Point(i32);

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "p{}", self.0)
    }
}

type Bumping = fn(&mut fmt::Formatter<'_>, &mut Point) -> fmt::Result;

struct Via(Writing, i32);

impl fmt::Display for Via {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (self.0)(f, self.1)?;
        let bump: Bumping = bumped;
        let mut p = Point(self.1);
        bump(f, &mut p)
    }
}

fn main() {
    let via = Via(bracketed, -3);
    println!("{via}");
}
