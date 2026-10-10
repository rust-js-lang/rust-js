//@ compile-fail: `next_back()` of
// A generic iterator is a JS iterator, which gives its items from the
// front only: its `next_back()` is refused.
fn ends<T: DoubleEndedIterator<Item = i32>>(mut t: T) -> (Option<i32>, Option<i32>) {
    (t.next(), t.next_back())
}

fn main() {
    println!("{:?}", ends(vec![1, 2, 3].into_iter()));
}
