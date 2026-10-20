// A generic iterator given as it is, which a type holds, is stepped by its
// `Iterator` dictionary (ADR 0411), and from its back by its
// `DoubleEndedIterator` dictionary, `{ next_back }` (ADR 0487). One given
// only itself knows its back (ADR 0429).
struct Ends<I> {
    inner: I,
}

fn back<I: Iterator<Item = i32> + DoubleEndedIterator>(ends: &mut Ends<I>) -> Option<i32> {
    ends.inner.next_back()
}

fn main() {
    let mut ends = Ends { inner: vec![1, 2, 3].into_iter() };
    println!("{:?}", back(&mut ends));
    println!("{:?} {:?}", back(&mut ends), ends.inner.next());
    let mut range = Ends { inner: 0..2 };
    println!("{:?} {:?}", back(&mut range), range.inner);
}
