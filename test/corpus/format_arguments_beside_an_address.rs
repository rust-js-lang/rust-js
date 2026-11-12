// A `println!` of locals, a `Vec`'s, a `String`'s, a tuple list's, in a crate
// that asks an address: the borrow of each later argument is recorded where
// it's made, which holds what's made before it, and an argument already
// made, `Argument::new_debug(&v)`, is kept for the format it's in, as a
// `&dyn Debug` is, never made a value (#1294). Its width from another
// argument, `{:>1$}`, and an argument made by a call that prints, each held
// in turn, as Rust evaluates each before it formats any.
use std::collections::BTreeSet;

#[repr(C)]
struct Record {
    a: u8,
    b: u32,
    c: u16,
}

fn record(n: u32) -> Record {
    Record { a: n as u8, b: n, c: n as u16 }
}

fn loud(n: usize) -> usize {
    println!("made {n}");
    n
}

fn main() {
    let v = vec![record(3), record(4), record(5)];
    let mut seen = Vec::new();
    for item in &v {
        seen.push(&raw const item.c as usize - v.as_ptr() as usize);
    }
    println!("{seen:?} {} {}", v[0].a, v[1].b);

    let numbers = vec![0, 1];
    let pairs: Vec<(i32, char)> = vec![(2, 'x')];
    let mut text = String::new();
    text.extend(BTreeSet::from(['c', 'a']));
    println!("{numbers:?} {pairs:?} {text}");
    println!("[{text:>width$}] {:?}", pairs.len(), width = loud(5));
    println!("{:>1$}|{numbers:?}|{text:?}", loud(2), loud(4));
}
