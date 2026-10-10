// A `BinaryHeap`'s `append`, `retain`, `drain` and `as_slice`, each leaving
// its items in the order std's heap keeps them, which `{:?}` shows.

use std::collections::BinaryHeap;

fn main() {
    let mut small: BinaryHeap<i32> = [5, 1, 8].into_iter().collect();
    let mut big: BinaryHeap<i32> = [3, 9, 2, 7, 4, 6].into_iter().collect();
    small.append(&mut big);
    println!("{:?} {:?}", small.as_slice(), big.len());
    let mut long: BinaryHeap<i32> = (0..40).map(|i| (i * 7) % 23).collect();
    let mut short: BinaryHeap<i32> = [50, 0].into_iter().collect();
    long.append(&mut short);
    println!("{:?}", long.as_slice());
    let mut left: BinaryHeap<i32> = (0..32).map(|i| (i * 5) % 17).collect();
    let mut right: BinaryHeap<i32> = (0..31).map(|i| (i * 3) % 19).collect();
    left.append(&mut right);
    println!("{:?}", left.as_slice());
    long.retain(|&x| x % 3 != 0);
    println!("{:?}", long.as_slice());
    let mut seen = Vec::new();
    small.retain(|&x| {
        seen.push(x);
        x > 3
    });
    println!("{seen:?} {:?}", small.as_slice());
    let mut sifted: BinaryHeap<i32> = [3, 0, 28, 25, 22, 19].into_iter().collect();
    let mut few: BinaryHeap<i32> = [5, 20, 6].into_iter().collect();
    sifted.append(&mut few);
    println!("{:?}", sifted.as_slice());
    let mut late: BinaryHeap<i32> = [7, 72, 137, 202, 56, 121, 186, 40].into_iter().collect();
    late.retain(|&x| x != 40 && x != 56);
    println!("{:?}", late.as_slice());
    let drained: Vec<i32> = small.drain().collect();
    println!("{drained:?} {}", small.is_empty());
    let words: BinaryHeap<&str> = ["pear", "fig", "apple"].into_iter().collect();
    println!("{:?} {:?}", words.as_slice(), words.peek());
}
