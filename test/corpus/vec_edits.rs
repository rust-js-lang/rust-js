// A `Vec`'s capacity, which a JS array has none of, so asking for some does
// nothing, and its edits: `swap_remove`, `resize`, `dedup_by`, `splice`,
// `pop_if`, as std has them (ADR 0315).

use std::collections::VecDeque;

fn main() {
    let mut v: Vec<u32> = Vec::with_capacity(8);
    v.reserve(4);
    v.reserve_exact(2);
    v.extend([5, 1, 4, 1, 3]);
    v.shrink_to_fit();
    v.shrink_to(2);
    // What it's given still runs.
    let asked = std::cell::Cell::new(0);
    let ask = || {
        asked.set(asked.get() + 1);
        asked.get()
    };
    v.reserve(ask());
    let w: Vec<u32> = Vec::with_capacity(ask());
    println!("{:?} {:?} {}", v, w, asked.get());
    let gone = v.swap_remove(1);
    println!("{gone} {:?}", v);
    v.resize(7, 9);
    println!("{:?}", v);
    v.resize(3, 0);
    println!("{:?}", v);
    let mut n = 0;
    v.resize_with(5, || {
        n += 1;
        n * 10
    });
    println!("{:?}", v);
    let mut tens = vec![1u32, 2, 12, 13, 3, 30];
    tens.dedup_by_key(|x| *x / 10);
    println!("{:?}", tens);
    let mut words: Vec<String> = ["a", "A", "b", "B", "b"].map(String::from).to_vec();
    words.dedup_by(|a, b| a.to_lowercase() == b.to_lowercase());
    println!("{:?}", words);
    let mut s = vec![1u32, 2, 3];
    s.extend_from_within(1..);
    println!("{:?}", s);
    let removed: Vec<u32> = s.splice(1..3, [7, 8, 9]).collect();
    println!("{:?} {:?}", removed, s);
    s.splice(..1, []);
    println!("{:?}", s);
    let top = s.pop_if(|x| *x > 5);
    let kept = s.pop_if(|x| *x > 100);
    println!("{top:?} {kept:?} {:?}", s);
    // What a closure changes through its `&mut` is kept, popped or not.
    let still = s.pop_if(|x| {
        *x += 1;
        false
    });
    let mut counts = vec![1u32, 1, 2];
    counts.dedup_by(|a, b| {
        *b += 10;
        a == b
    });
    println!("{still:?} {:?} {:?}", s, counts);
    let boxed: Box<[u32]> = s.into_boxed_slice();
    println!("{:?}", boxed);
    let grid = vec![[1u32, 2], [3, 4]];
    println!("{:?}", grid.into_flattened());
    let mut points = vec![vec![0u32]; 1];
    points.resize(3, vec![1]);
    points[1].push(2);
    println!("{:?}", points);
    let mut q: VecDeque<u32> = VecDeque::with_capacity(4);
    q.reserve(2);
    q.push_back(1);
    q.shrink_to_fit();
    q.resize(3, 7);
    println!("{:?}", q);
}
