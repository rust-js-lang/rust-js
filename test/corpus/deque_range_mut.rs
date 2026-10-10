// A `VecDeque`'s `range_mut`: a `&mut` to each item in a range, in order.
use std::collections::VecDeque;

fn main() {
    let mut d: VecDeque<i32> = (1..=6).collect();
    d.push_front(0);
    for x in d.range_mut(2..5) {
        *x *= 10;
    }
    println!("{d:?}");
    let mut words: VecDeque<String> = ["a", "b", "c"].map(String::from).into();
    words.range_mut(1..).for_each(|w| w.push('!'));
    let mut points: VecDeque<(i32, i32)> = [(1, 1), (2, 2)].into();
    for p in points.range_mut(..1) {
        p.0 = 9;
    }
    let mut it = d.range_mut(..=1);
    if let Some(first) = it.next() {
        *first -= 1;
    }
    println!("{words:?} {points:?} {} {}", it.count(), d.range_mut(3..=3).count());
    println!("{d:?}");
}
