//@ compile-fail: a `std::boxed::Box<std::ops::Range<i32>>` as an `ExactSizeIterator`, which rust-js can't count yet
// A boxed range taken through a `&mut` of its box, in a crate that reads a
// capacity: `Box<Range<i32>>` is an `ExactSizeIterator`, so std grows the
// `Vec` by its `take(2)`'s exact hint, 2 (#1892). Its box's form, not its
// type, says whether JS's hint keeps that: refused till it's read
// (#1892), never another capacity. Natively `[0, 1] 4` and `[2, 3, 4] 4`.
fn main() {
    let mut boxed: Box<std::ops::Range<i32>> = Box::new(0..5);
    let taken: Vec<i32> = (&mut boxed).take(2).collect();
    println!("{:?} {}", taken, taken.capacity());
    let rest: Vec<i32> = boxed.collect();
    println!("{:?} {}", rest, rest.capacity());
}
