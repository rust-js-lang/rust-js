// `v.borrow_mut()` of std's `BorrowMut<[T]>` of a `Vec` or an array, a
// type known where it's called: the same array, which a write through it
// changes in place, as `as_mut()` is, where it read generic code's box of
// it, `v.value`, and threw (#807).
use std::borrow::BorrowMut;

fn first(v: &mut Vec<usize>) -> usize {
    let s: &mut [usize] = v.borrow_mut();
    s[0] += 1;
    s.len()
}

fn main() {
    let mut v = vec![1, 2];
    println!("{} {:?}", first(&mut v), v);
    let mut a = [3u8, 4, 5];
    let s: &mut [u8] = a.borrow_mut();
    s[2] = 9;
    println!("{:?}", a);
    let mut w = vec![String::from("x")];
    let t: &mut [String] = w.borrow_mut();
    t[0].push('y');
    println!("{:?}", w);
}
