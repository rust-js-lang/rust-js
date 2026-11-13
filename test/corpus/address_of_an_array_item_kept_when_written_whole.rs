// An array an item of which a raw pointer is taken to, by an index read at
// run time, assigned whole again: each item a pointer may be at written
// over by the new array's, so the pointer reads the new item where it was,
// as Rust's does, and its address is the same (#1553). Where the write
// kept the item at the index local of the pointer's own statement, which
// the write never declares, it threw a `ReferenceError`. And an array moved
// out of it first is its own, each item copied apart, which the write over
// the place it left changes nothing of.
#[derive(Debug)]
struct N(u8, u32);

fn at(i: usize) -> usize {
    i
}

fn main() {
    let mut items = [N(30, 31), N(32, 33)];
    let first = &raw const items[0] as usize;
    let second = &raw const items[at(1)];
    items = [N(34, 35), N(36, 37)];
    println!("{} {items:?}", first == &raw const items[0] as usize);
    println!("{:?}", unsafe { &*second });
    items = [N(38, 39), N(40, 41)];
    println!("{:?} {items:?}", unsafe { &*second });
    let moved = items;
    items = [N(42, 43), N(44, 45)];
    println!("{moved:?} {items:?}");
}
