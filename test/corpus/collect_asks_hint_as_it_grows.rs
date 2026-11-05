// A `collect()` into a `Vec` of what isn't `TrustedLen` asks its hint
// again each time the `Vec` is full, as std's `extend_desugared` does: its
// capacity starts at the hint's lower bound and one more, or more, by the
// items' size (`MIN_NON_ZERO_CAP`: 8 of 1 byte, 4 of up to 1024, else 1),
// and doubles, or grows to what the hint asks then (#728). Of items of no
// size it's never full. Where a hint asked as it grows panics, the item
// just taken is dropped, then what's left, then what it collected.
use std::panic::{AssertUnwindSafe, catch_unwind};

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

/// A crate's iterator of `end` items made by `make`, whose `size_hint`
/// prints, says `lower` of what's left, and panics at `panics_at`.
struct Own<F> {
    at: u32,
    end: u32,
    lower: fn(u32) -> usize,
    panics_at: u32,
    make: F,
}

impl<F> Drop for Own<F> {
    fn drop(&mut self) {
        println!("drop own at {}", self.at);
    }
}

impl<T, F: FnMut(u32) -> T> Iterator for Own<F> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.at += 1;
        (self.at <= self.end).then(|| (self.make)(self.at))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        println!("hint at {}", self.at);
        if self.at == self.panics_at {
            panic!("hint");
        }
        let left = self.end - self.at.min(self.end);
        ((self.lower)(left), None)
    }
}

fn own<T>(end: u32, lower: fn(u32) -> usize, make: impl FnMut(u32) -> T) -> Own<impl FnMut(u32) -> T> {
    Own { at: 0, end, lower: lower as fn(u32) -> usize, panics_at: 0, make }
}

fn none(_: u32) -> usize {
    0
}

fn all(left: u32) -> usize {
    left as usize
}

fn half(left: u32) -> usize {
    left as usize / 2
}

fn run(name: &str, f: impl FnOnce() -> usize) {
    println!("{name}");
    let r = catch_unwind(AssertUnwindSafe(f));
    println!("{name} {:?}", r.ok());
}

fn main() {
    run("bytes", || own(20, none, |n| n as u8).fuse().collect::<Vec<u8>>().len());
    run("words", || own(20, none, |n| n).fuse().collect::<Vec<u32>>().len());
    run("words, half hinted", || own(20, half, |n| n).fuse().collect::<Vec<u32>>().len());
    run("words, all hinted", || own(20, all, |n| n).fuse().collect::<Vec<u32>>().len());
    run("big", || own(5, none, |n| [n as u8; 2000]).fuse().collect::<Vec<_>>().len());
    run("of no size", || own(10, none, |_| ()).fuse().collect::<Vec<()>>().len());
    run("peeked", || own(10, none, |n| n as u16).peekable().collect::<Vec<u16>>().len());
    run("dropping, a hint panics as it grows", || {
        let mut it = own(9, none, D);
        it.panics_at = 5;
        it.fuse().collect::<Vec<D>>().len()
    });
}
