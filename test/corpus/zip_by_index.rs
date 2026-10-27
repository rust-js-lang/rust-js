// A zip of two `TrustedRandomAccess` iterators, a slice's, a range's, a
// `map` of one, is std's by index: it pairs up to the shorter's length, so
// the longer's closure never runs for an item past it, whether it's
// stepped by `next()`, `nth` or `collect()`. A side that's no such
// iterator, a `filter`'s, steps as the general zip does, its closure run
// once more where the other's run out (ADR 0973).
fn noisy(tag: &str, x: i32) -> i32 {
    println!("{tag}{x}");
    x
}

fn main() {
    let v = [1, 2, 3];
    let w = [10, 20];
    let z: Vec<(i32, &i32)> = v.iter().map(|&x| noisy("a", x)).zip(w.iter()).collect();
    println!("{z:?}");
    for (x, y) in v.iter().map(|&x| noisy("b", x)).zip(w.iter()) {
        println!("{x} {y}");
    }
    let mut it = w.iter().zip(v.iter().map(|&x| noisy("c", x)));
    while let Some(p) = it.next() {
        println!("{p:?}");
    }
    let n = v.iter().map(|&x| noisy("e", x)).zip(w.iter()).nth(1);
    println!("{n:?}");
    let ranged: Vec<(i32, i32)> = (0..5).map(|x| noisy("r", x)).zip(1..3).collect();
    println!("{ranged:?}");
    let both: Vec<(i32, i32)> = v
        .iter()
        .map(|&x| noisy("l", x))
        .zip(w.iter().map(|&y| noisy("m", y)))
        .collect();
    println!("{both:?}");
    let filtered: Vec<(i32, &i32)> = v
        .iter()
        .map(|&x| noisy("f", x))
        .filter(|_| true)
        .zip(w.iter())
        .collect();
    println!("{filtered:?}");
    let borrowed: Vec<(i32, &i32)> = v.iter().map(|&x| noisy("g", x)).zip(&w).collect();
    println!("{borrowed:?}");
    let called: Vec<(i32, &i32)> = std::iter::zip(v.iter().map(|&x| noisy("i", x)), &w).collect();
    println!("{called:?}");
    let pure: Vec<(&i32, &i32)> = v.iter().zip(w.iter()).collect();
    println!("{pure:?}");
}
