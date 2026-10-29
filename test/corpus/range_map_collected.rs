// `(0..n).map(f).collect()` into a `Vec` is `Array.from({ length: n }, (_, i)
// => ..)`, room for its items asked first, as a person makes `n` items
// (#361). Each item its own, a fresh `Vec` each; of another range, from 1,
// inclusive or reversed, or into a heap, as before.

use std::collections::BinaryHeap;

fn rows(n: usize) -> Vec<Vec<u8>> {
    (0..n).map(|_| Vec::new()).collect()
}

fn main() {
    let none: Vec<Option<u8>> = (0..3).map(|_| None).collect();
    println!("{none:?}");
    let squares: Vec<usize> = (0..5).map(|i| i * i).collect();
    println!("{squares:?}");
    let mut rows = rows(3);
    rows[0].push(1);
    println!("{rows:?}");
    let from_one: Vec<u32> = (1..4).map(|i| i * 10).collect();
    println!("{from_one:?}");
    let inclusive: Vec<u32> = (0..=3).map(|i| i + 1).collect();
    println!("{inclusive:?}");
    let back: Vec<u32> = (0..4).rev().map(|i| i * 2).collect();
    println!("{back:?}");
    let heap: BinaryHeap<u32> = (0..5).map(|i| (i * 7) % 5).collect();
    println!("{:?}", heap.into_sorted_vec());
    let mut seen = Vec::new();
    let told: Vec<u32> = (0..3)
        .map(|i| {
            seen.push(i);
            i
        })
        .collect();
    println!("{told:?} {seen:?}");
}
