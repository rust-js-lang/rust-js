// `rev()` of a lazy chain over a slice: Rust's `Map` and `Filter` call
// their closures from the back as they're stepped from it, so the chain is
// made of the items reversed, each closure called on the same items in the
// same order, `v.toReversed().map(f)`; an `enumerate`'s index is its
// item's, made before the items are reversed (ADR 0164).
fn shout(x: &i32) -> i32 { println!("map {x}"); x * 2 }
fn main() {
    let v = vec![1, -2, 3, 4];
    let a: Vec<i32> = v.iter().map(shout).rev().collect();
    let b: Vec<i32> = v.iter().filter(|x| { println!("test {x}"); **x > 0 }).rev().copied().collect();
    let c: Vec<i32> = v.iter().map(|x| { println!("m {x}"); x + 1 }).filter(|x| { println!("f {x}"); x % 2 == 0 }).rev().take(1).collect();
    let mut it = v.iter().map(|x| x * 10).rev();
    let first = it.next();
    let rest: Vec<i32> = it.collect();
    println!("{a:?} {b:?} {c:?} {first:?} {rest:?}");
    more();
}
fn more() {
    let v = vec![5, 6, 7];
    let e: Vec<(usize, i32)> = v.iter().copied().enumerate().rev().collect();
    let f: Vec<(usize, &i32)> = v.iter().enumerate().rev().skip(1).collect();
    let g: Vec<i32> = v.iter().enumerate().map(|(i, x)| { println!("g {i}"); x * i as i32 }).rev().collect();
    println!("{e:?} {f:?} {g:?}");
}
