// A VecDeque's as_slices() and as_mut_slices(): its items, in order, the
// first all of them, the second none; writes through them its own.
use std::collections::VecDeque;
fn main() {
    let mut d: VecDeque<i32> = (1..=5).collect();
    d.rotate_left(2);
    d.push_front(0);
    let (a, b) = d.as_slices();
    println!("{} {}", a.len() + b.len(), a.iter().chain(b).sum::<i32>());
    let all: Vec<i32> = a.iter().chain(b.iter()).copied().collect();
    println!("{all:?}");
    let (x, y) = d.as_mut_slices();
    for v in x.iter_mut().chain(y.iter_mut()) {
        *v *= 10;
    }
    println!("{d:?}");
    let e: VecDeque<String> = VecDeque::new();
    let (p, q) = e.as_slices();
    println!("{} {}", p.is_empty(), q.is_empty());
}
