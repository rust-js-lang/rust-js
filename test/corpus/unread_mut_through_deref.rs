//@ library-differs
// A `&mut` nothing reads, `let _ = &mut p.y` through `deref_mut`: what makes
// its place runs, the counter's count, and no handle is made on it.
use std::ops::{Deref, DerefMut};

struct Counted<T> {
    muts: usize,
    value: T,
}

impl<T> Deref for Counted<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T> DerefMut for Counted<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.muts += 1;
        &mut self.value
    }
}

struct Point {
    x: i32,
    y: i32,
}

fn main() {
    let mut p = Counted { muts: 0, value: Point { x: 1, y: 2 } };
    let _ = &mut p.y;
    p.x += 3;
    println!("{} {} {}", p.muts, p.x, p.y);
}
