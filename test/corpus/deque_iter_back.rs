// A `VecDeque`'s iterator is a `$iter` of its array, as a slice's is: it
// steps from its back and knows its length, `next_back()` and `len()`, as
// every iterator of an array does (the second architecture review: one
// list said so of a slice's and not of a deque's).

use std::collections::VecDeque;

fn main() {
    let deque: VecDeque<i32> = VecDeque::from(vec![1, 2, 3, 4]);
    let mut items = deque.iter();
    let last = items.next_back();
    let first = items.next();
    println!("{:?} {:?} {}", last, first, items.len());
    let mut values = vec![4, 5, 6];
    let mut each = values.iter_mut();
    if let Some(x) = each.next_back() {
        *x += 10;
    }
    let left = each.len();
    println!("{:?} {}", values, left);
}
