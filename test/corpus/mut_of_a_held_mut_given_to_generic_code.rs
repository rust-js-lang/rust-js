// A `&mut` to a local that holds a `&mut` to an object, given where generic
// code has a box of its type parameter, `&mut W` of `&mut by` (#1300): the
// box holds the object's in-place cell, as a `W` given `&mut v` by value
// does, and what's copied back is the `&mut` that holds; one held in a
// cell, an `Option`'s, is read through. Of a writer, a reader, a struct's
// field, an array's item, an `Option`'s payload, and one generic code
// replaces whole.
use std::io::{Cursor, Read, Write};

fn put<W: Write>(w: &mut W) {
    w.write_all(b"x").unwrap();
}

fn put_by_value<W: Write>(mut w: W) {
    w.write_all(b"y").unwrap();
}

fn first<R: Read>(r: &mut R) -> u8 {
    let mut b = [0u8; 1];
    r.read_exact(&mut b).unwrap();
    b[0]
}

fn replace_with<T>(slot: &mut T, other: T) {
    *slot = other;
}

struct Holder<'a> {
    out: &'a mut Vec<u8>,
}

fn main() {
    let mut v = Vec::new();
    put_by_value(&mut v);
    let mut by = &mut v;
    put(&mut by);
    put(&mut by);
    let mut h = Holder { out: by };
    put(&mut h.out);
    println!("{v:?}");
    let mut w = Vec::new();
    let mut slot = Some(&mut w);
    put(slot.as_mut().unwrap());
    println!("{w:?}");

    let mut c = Cursor::new(vec![7u8, 8]);
    let mut rc = &mut c;
    println!("{} {} {}", first(&mut rc), first(&mut rc), c.position());

    let mut list = [Vec::new(), Vec::new()];
    {
        let [one, two] = &mut list;
        let mut pair = [one, two];
        put(&mut pair[1]);
    }
    println!("{list:?}");

    let mut a = vec![1];
    let mut b = vec![2];
    let mut r = &mut a;
    replace_with(&mut r, &mut b);
    r.push(3);
    println!("{a:?} {b:?}");
}
