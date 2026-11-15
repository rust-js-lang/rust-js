// A value of a type parameter whose `Copy` is a supertrait's is copied as
// one of a `T: Copy` is (#1925): `let mut copy = value` is its own value,
// so what changes it leaves `value` as it was, through `trait Copied:
// Copy`, a trait that extends that, and a where-clause's bound alike; its
// dictionary carries `Copy`'s, `TCopied.Copy()`. A `Clone` and an
// `Iterator` a supertrait gives are read through it too.
use std::fmt::Debug;

trait Copied: Copy {}

trait Deeper: Copied {}

trait Dup: Clone {}

trait Stream: Iterator<Item = u32> {}

#[derive(Clone, Copy, Debug)]
struct Point {
    x: u32,
}

impl Copied for Point {}

impl Deeper for Point {}

impl Dup for Point {}

impl Copied for u32 {}

struct Count(u32);

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 < 3 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }
}

impl Stream for Count {}

fn bumped<T: Copied + Debug>(value: T, change: impl Fn(&mut T)) -> String {
    let mut copy = value;
    change(&mut copy);
    format!("{value:?} {copy:?}")
}

fn deeper<T: Deeper + Debug>(value: T, change: impl Fn(&mut T)) -> String {
    let mut copy = value;
    change(&mut copy);
    format!("{value:?} {copy:?}")
}

fn where_bound<T>(value: T, change: impl Fn(&mut T)) -> String
where
    T: Copied + Debug,
{
    let mut copy = value;
    change(&mut copy);
    format!("{value:?} {copy:?}")
}

fn duped<T: Dup + Debug>(value: &T, change: impl Fn(&mut T)) -> String {
    let mut copy = value.clone();
    change(&mut copy);
    format!("{value:?} {copy:?}")
}

fn summed<S: Stream>(stream: S) -> u32 {
    let mut total = 0;
    for x in stream {
        total += x;
    }
    total
}

fn main() {
    println!("{}", bumped(Point { x: 1 }, |p| p.x += 1));
    println!("{}", bumped(5u32, |n| *n += 1));
    println!("{}", deeper(Point { x: 1 }, |p| p.x += 1));
    println!("{}", where_bound(Point { x: 1 }, |p| p.x += 1));
    println!("{}", duped(&Point { x: 1 }, |p| p.x += 1));
    println!("{}", summed(Count(0)));
}
