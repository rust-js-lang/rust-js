// `rev()` of a lazy chain over a range: the range stepped from its end, each
// closure called from the back as Rust's `next_back` calls it, so a `take`
// after it calls them only as often as Rust does (ADR 0164).
fn main() {
    let v: Vec<_> = (1..4).map(|x| x * 2).rev().collect();
    let w: Vec<_> = (1..=6).filter(|x| x % 2 == 0).map(|x| x + 1).rev().collect();
    let mut seen = Vec::new();
    let z: Vec<_> = (0..4)
        .map(|x| {
            seen.push(x);
            x
        })
        .rev()
        .take(2)
        .collect();
    let last = (10..20).filter(|x| x % 3 == 0).rev().next();
    let empty: Vec<u32> = (5..5).map(|x| x + 1).rev().collect();
    println!("{v:?} {w:?} {z:?} {seen:?} {last:?} {empty:?}");
}
