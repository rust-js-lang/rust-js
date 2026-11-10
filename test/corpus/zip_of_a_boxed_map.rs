// A zip of a map whose function is boxed, `Box<dyn FnMut>`, over a slice's
// items, which std folds by index to the shorter side, so the function is
// never called for an item past it, as a closure isn't; and of one an
// `impl Trait` hides, reversed or not, whose type native Rust's
// specialization sees (#1225).
use std::cell::Cell;

fn counted<'a>(v: &'a [i32], calls: &'a Cell<u32>) -> impl DoubleEndedIterator<Item = i32> + ExactSizeIterator + 'a {
    v.iter().map(move |&x| {
        calls.set(calls.get() + 1);
        println!("b{x}");
        x
    })
}

fn outer<'a>(v: &'a [i32], calls: &'a Cell<u32>) -> impl DoubleEndedIterator<Item = i32> + ExactSizeIterator + 'a {
    counted(v, calls).rev()
}

fn main() {
    let mut calls = 0;
    let f: Box<dyn FnMut(&i32) -> i32> = Box::new(|&x| {
        calls += 1;
        x
    });
    let pairs: Vec<(i32, i32)> = [1, 2, 0].iter().map(f).zip(vec![7, 8]).collect();
    println!("{pairs:?} {calls}");
    let mut seen = Vec::new();
    let g: Box<dyn FnMut(&u8) -> u8> = Box::new(|&x| {
        seen.push(x);
        x + 1
    });
    let n = [5u8, 6, 7, 8].iter().map(g).zip(['a', 'b']).count();
    println!("{n} {seen:?}");
    let calls = Cell::new(0);
    let pairs: Vec<(i32, i32)> = counted(&[1, 2, 3], &calls).zip(vec![7, 8]).collect();
    println!("{pairs:?} {}", calls.get());
    let calls = Cell::new(0);
    let pairs: Vec<(i32, i32)> = outer(&[1, 2, 3], &calls).zip(vec![7, 8]).collect();
    println!("{pairs:?} {}", calls.get());
    let calls = Cell::new(0);
    let n = vec![5, 6]
        .into_iter()
        .zip(counted(&[1, 2, 3], &calls))
        .fold(0, |a, (x, y)| a + x + y);
    println!("{n} {}", calls.get());
}
