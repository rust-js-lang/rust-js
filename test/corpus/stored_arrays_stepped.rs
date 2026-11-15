// An array a constant, a static or a thread-local stores, a `for` loop
// steps through: a constant's of items no part of which is an object read
// as it is, as a library too, nothing else holding or changing it; one the
// loop's body changes, a `static mut`'s or a cell's, read as it was when
// the loop began; and a literal's bytes, `*b"ab"`, the array written there,
// a copy of it changed apart (#1923).
use std::cell::Cell;

const SHAPES: [&str; 3] = ["random", "few", "sorted"];
static mut COUNTS: [u32; 3] = [1, 2, 3];

thread_local! {
    static SEEN: Cell<[u32; 3]> = const { Cell::new([4, 5, 6]) };
}

fn bump() {
    unsafe {
        COUNTS[2] += 10;
    }
}

fn main() {
    for shape in SHAPES {
        print!("{shape} ");
    }
    println!();
    for n in unsafe { COUNTS } {
        bump();
        print!("{n} ");
    }
    println!("{:?}", unsafe { COUNTS });
    for n in SEEN.get() {
        SEEN.set([0, 0, 0]);
        print!("{n} ");
    }
    println!("{:?}", SEEN.get());
    let bytes = *b"ab";
    let mut copy = bytes;
    copy[0] = b'z';
    println!("{bytes:?} {copy:?}");
}
