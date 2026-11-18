// A `&mut` to a range given to generic code bound by `ExactSizeIterator`
// or `DoubleEndedIterator`: what it lends made knowing its length and its
// back, as std's `&mut I` passes `len` and `next_back` on to its iterator
// (#2075, ADR 0937), stepped where it is, so the owner sees each step after:
// a range partly stepped before it's lent, stepped from its back through
// the `&mut` and then from its front by its owner; and an inclusive one. A
// `Box`'s stays refused (#1892): std's `Box<I>` isn't `TrustedLen`.
fn len_of<I: ExactSizeIterator>(it: I) -> usize {
    it.len()
}

fn back_of<I: DoubleEndedIterator>(mut it: I) -> Option<I::Item> {
    it.next_back()
}

fn both<I: DoubleEndedIterator + ExactSizeIterator>(mut it: I) -> (usize, Option<I::Item>, usize) {
    let before = it.len();
    let back = it.next_back();
    (before, back, it.len())
}

fn main() {
    let mut r = 0..6u32;
    println!("{:?}", r.next());
    println!("{}", len_of(&mut r));
    println!("{:?}", back_of(&mut r));
    println!("{:?} {:?}", r.next(), r);
    println!("{:?}", both(&mut r));
    println!("{:?} {}", r, r.len());

    let mut s = 1..=4u8;
    println!("{}", len_of(&mut s));
    println!("{:?}", back_of(&mut s));
    println!("{:?} {:?}", s.next(), s);

    println!("end");
}
