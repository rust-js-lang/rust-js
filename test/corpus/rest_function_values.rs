// A function of rest arguments, JS's `function total(...xs)` (ADR 0664),
// as a value Rust calls with a slice: a `fn` pointer, an `impl Fn`, a
// generic's, a box's, a struct's field, an iterator's callback and a
// variable of it. Each call gives the slice's items, however it reaches
// the function; only where JS alone reads the function, `js::function`'s
// members, is the value the function itself (#540). A closure that passes
// what it took and the slice it's given on to it gives the same.

fn total(#[cfg_attr(rust_js, rust_js::rest)] xs: &[u32]) -> u32 {
    xs.iter().sum()
}

fn offset(by: u32, #[cfg_attr(rust_js, rust_js::rest)] xs: &[u32]) -> u32 {
    by * 100 + total(xs)
}

fn apply(f: impl Fn(&[u32]) -> u32, xs: &[u32]) -> u32 {
    f(xs)
}

fn apply_generic<F: Fn(u32, &[u32]) -> u32>(f: F, by: u32, xs: &[u32]) -> u32 {
    f(by, xs)
}

struct Reducer {
    reduce: fn(&[u32]) -> u32,
}

fn main() {
    let xs = [1, 2, 3];
    let pointer: fn(&[u32]) -> u32 = total;
    let named = total;
    let boxed: Box<dyn Fn(u32, &[u32]) -> u32> = Box::new(offset);
    let reducer = Reducer { reduce: total };
    println!(
        "{} {} {} {} {} {}",
        pointer(&xs),
        named(&xs[1..]),
        apply(total, &xs),
        apply_generic(offset, 2, &xs),
        boxed(3, &[]),
        (reducer.reduce)(&[4, 5]),
    );

    let rows: Vec<Vec<u32>> = vec![vec![], vec![7], vec![1, 1, 1]];
    let sums: Vec<u32> = rows.iter().map(Vec::as_slice).map(total).collect();
    println!("{sums:?}");

    let by = 5;
    let shifted = move |xs: &[u32]| offset(by, xs);
    println!("{} {}", shifted(&xs), shifted(&[9]));

    let chosen: Option<fn(&[u32]) -> u32> = Some(total);
    println!("{:?}", chosen.map(|f| f(&[10, 20])));
}
