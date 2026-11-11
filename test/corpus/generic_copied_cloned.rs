// A clone of a `copied()` or a `cloned()` of generic code's items: made by
// a function of its source, which its clone calls again with a clone of
// it, so the clone starts where the original is and copies each item
// again, as std's `Copied` and `Cloned` clone their source (ADR 0525). A
// struct's copies are its own, of concrete items too. Each was refused,
// "whose items are copied as it steps" (#1181).
use std::fmt::Debug;

#[derive(Clone, Copy, Debug)]
struct P(u8);

fn copied_twice<T: Copy + Debug>(v: &[T]) -> (Vec<T>, Vec<T>) {
    let it = v.iter().copied();
    let again = it.clone();
    (it.collect(), again.collect())
}

fn copied_after<T: Copy + Debug>(v: &[T]) -> (Option<T>, Vec<T>, Vec<T>) {
    let mut it = v.iter().copied();
    let first = it.next();
    let rest = it.clone();
    (first, it.collect(), rest.collect())
}

fn cloned_twice<T: Clone + Debug>(v: &[T]) -> (Vec<T>, Vec<T>) {
    let mut it = v.iter().cloned();
    it.next();
    let again = it.clone();
    (it.collect(), again.collect())
}

fn main() {
    println!("{:?}", copied_twice(&[1i8, 2, 3]));
    println!("{:?}", copied_twice(&[(1u8, 'a'), (2, 'b')]));
    println!("{:?}", copied_after(&[4u32, 5, 6]));
    let ps = [P(1), P(2), P(3)];
    let (first, mut rest, mut again) = copied_after(&ps);
    rest[0].0 = 20;
    again[0].0 = 30;
    println!("{first:?} {rest:?} {again:?} {ps:?}");
    let mut it = ps.iter().copied();
    it.next();
    let later = it.clone();
    let mut a: Vec<P> = it.collect();
    a[0].0 = 9;
    println!("{a:?} {:?} {ps:?}", later.collect::<Vec<_>>());
    let words = [String::from("a"), String::from("b"), String::from("c")];
    let (mut left, right) = cloned_twice(&words);
    left[0].push('!');
    println!("{left:?} {right:?} {words:?}");
}
